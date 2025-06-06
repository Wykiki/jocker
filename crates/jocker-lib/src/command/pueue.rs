use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    process::Stdio,
    time::Duration,
};

use pueue_lib::{
    network::message::{
        AddRequest, GroupRequest, KillRequest, LogRequest, ResetRequest, ResetTarget, Signal,
        StreamRequest, TaskSelection,
    },
    Client, Group, Request, Response, Settings, Task, TaskStatus,
};
use snap::read::FrameDecoder;
use tokio::{
    process::{Child, Command},
    sync::{mpsc::Sender, Mutex},
    time::sleep,
};

use crate::error::{Error, InnerError, Result};

pub(crate) struct Pueue {
    group: String,
    client: Mutex<Client>,
}

impl Pueue {
    pub(crate) async fn new(project_id: &str) -> Result<Self> {
        // Try to start pueued if initial client creation fails
        let mut client = match Self::client().await {
            Ok(client) => client,
            Err(_) => {
                Pueued::daemonize().await?;
                Self::client().await?
            }
        };
        let group = Self::init_or_get_group(&mut client, project_id).await?;
        Ok(Self {
            group,
            client: Mutex::new(client),
        })
    }

    pub(crate) async fn client() -> Result<Client> {
        let (settings, _) = Settings::read(&None)?;
        let client = Client::new(settings, true)
            .await
            .map_err(|e| InnerError::Pueue(pueue_lib::Error::Generic(e.to_string())))?;
        Ok(client)
    }

    pub(crate) async fn start(
        &self,
        process_name: String,
        command: String,
        path: PathBuf,
        envs: HashMap<String, String>,
    ) -> Result<usize> {
        let mut client = self.client.lock().await;
        client
            .send_request(Request::Add(AddRequest {
                command,
                path,
                envs,
                group: self.group.clone(),
                label: Some(process_name.clone()),
                ..Default::default()
            }))
            .await?;
        let rsp = client.receive_response().await?;
        let task_id = match rsp {
            Response::AddedTask(task) => task.task_id,
            e => {
                return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                    format!("{:?}", e),
                ))))
            }
        };
        drop(client);
        while !matches!(
            self.process_status(&task_id).await?,
            TaskStatus::Running { .. }
        ) {
            sleep(Duration::from_millis(100)).await;
        }
        Ok(task_id)
    }

    pub(crate) async fn processes(&self) -> Result<HashMap<String, (usize, TaskStatus)>> {
        Ok(self
            .processes_by_pid()
            .await?
            .into_iter()
            .map(|entry| {
                (
                    entry.1.label.clone().unwrap_or("NONE".to_string()),
                    (entry.1.id, entry.1.status.clone()),
                )
            })
            .collect())
    }

    async fn processes_by_pid(&self) -> Result<HashMap<usize, Task>> {
        let mut client = self.client.lock().await;
        client.send_request(Request::Status).await?;
        let rsp = client.receive_response().await?;
        match rsp {
            Response::Status(state) => {
                let task_ids = state.task_ids_in_group(&self.group);
                let tasks = state
                    .tasks
                    .into_iter()
                    .filter(|entry| task_ids.contains(&entry.0))
                    .map(|entry| (entry.1.id, entry.1))
                    .collect();
                Ok(tasks)
            }
            e => Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                format!("{:?}", e),
            )))),
        }
    }

    async fn process_status(&self, pid: &usize) -> Result<TaskStatus> {
        Ok(self
            .processes_by_pid()
            .await?
            .get(pid)
            .ok_or_else(|| {
                Error::new(InnerError::Pueue(pueue_lib::Error::Generic(format!(
                    "Cannot get status of process with pid {pid}"
                ))))
            })?
            .status
            .clone())
    }

    pub(crate) async fn logs(
        &self,
        log_tx: Sender<String>,
        process_prefix: &str,
        pid: usize,
        lines: Option<usize>,
        follow: bool,
    ) -> Result<()> {
        match follow {
            true => self.follow(log_tx, process_prefix, pid, lines).await,
            false => self.log(log_tx, process_prefix, pid, lines).await,
        }
    }

    async fn log(
        &self,
        log_tx: Sender<String>,
        process_prefix: &str,
        pid: usize,
        lines: Option<usize>,
    ) -> Result<()> {
        let mut client = self.client.lock().await;

        client
            .send_request(LogRequest {
                tasks: TaskSelection::TaskIds(vec![pid]),
                lines,
                send_logs: true,
            })
            .await?;
        let response = client.receive_response().await?;
        match response {
            Response::Log(response) => {
                for (_, text) in response {
                    let bytes = text.output.clone().unwrap_or_default();
                    let mut decompressor = FrameDecoder::new(bytes.as_slice());
                    let mut buf = vec![];
                    std::io::copy(&mut decompressor, &mut buf).unwrap();
                    let content = String::from_utf8(buf)?;
                    for line in content.lines() {
                        log_tx
                            .send(format!("{process_prefix}{}", line))
                            .await
                            .unwrap();
                    }
                }
            }
            other => {
                return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                    format!("Received unhandled Response message during logs streaming: {other:?}"),
                ))))
            }
        }
        Ok(())
    }

    async fn follow(
        &self,
        log_tx: Sender<String>,
        process_prefix: &str,
        pid: usize,
        lines: Option<usize>,
    ) -> Result<()> {
        // Create its own client to avoid blocking
        let mut client = Self::client().await?;
        client
            .send_request(StreamRequest {
                tasks: TaskSelection::TaskIds(vec![pid]),
                lines,
            })
            .await?;

        loop {
            let response = client.receive_response().await?;
            match response {
                Response::Stream(response) => {
                    for (_, text) in response.logs {
                        for line in text.lines() {
                            log_tx
                                .send(format!("{process_prefix}{}", line))
                                .await
                                .unwrap();
                        }
                    }
                }
                Response::Close => break,
                Response::Failure(text) => {
                    return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                        format!("Failure during logs streaming: {text}"),
                    ))))
                }
                other => {
                    return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                        format!(
                            "Received unhandled Response message during logs streaming: {other:?}"
                        ),
                    ))))
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn stop(&self, pid: usize, kill: bool) -> Result<()> {
        let signal = Some(if kill {
            Signal::SigKill
        } else {
            Signal::SigTerm
        });
        let mut client = self.client.lock().await;
        client
            .send_request(Request::Kill(KillRequest {
                tasks: TaskSelection::TaskIds(vec![pid]),
                signal,
            }))
            .await?;
        let rsp = client.receive_response().await?;
        if !rsp.success() {
            return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                format!("{:?}", rsp),
            ))));
        }
        drop(client);
        while !matches!(self.process_status(&pid).await?, TaskStatus::Done { .. }) {
            sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    pub(crate) async fn clean(self) -> Result<()> {
        self.reset_group(&self.group).await?;
        self.remove_group(&self.group).await
    }

    async fn init_or_get_group(client: &mut Client, project_id: &str) -> Result<String> {
        let group = format!("jocker-{project_id}");
        if !groups(client).await?.contains_key(&group) {
            add_group(client, &group).await?;
        }
        Ok(group)
    }

    async fn reset_group(&self, group: &str) -> Result<()> {
        let mut client = self.client.lock().await;
        client
            .send_request(ResetRequest {
                target: ResetTarget::Groups(vec![group.to_owned()]),
            })
            .await?;
        let response = client.receive_response().await?;
        if !response.success() {
            return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                format!("{:?}", response),
            ))));
        }
        drop(client);
        while !self.processes().await?.is_empty() {
            sleep(Duration::from_millis(100)).await;
            // TODO: Handle timeout
        }
        Ok(())
    }

    async fn remove_group(&self, group: &str) -> Result<()> {
        let mut client = self.client.lock().await;
        client
            .send_request(GroupRequest::Remove(group.to_owned()))
            .await?;
        let response = client.receive_response().await?;
        if !response.success() {
            return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                format!("{:?}", response),
            ))));
        }
        Ok(())
    }
}

pub(crate) struct Pueued;

impl Pueued {
    /// Launch `pueued` as a background daemon
    pub async fn daemonize() -> Result<Child> {
        let mut build = Command::new("pueued");
        build.stdout(Stdio::piped()).stderr(Stdio::piped());
        build.arg("-d");
        let build = build
            .spawn()
            .map_err(Error::with_context(InnerError::Pueue(
                pueue_lib::Error::Generic("Unable to start `pueued -d` command".to_string()),
            )))?;
        Ok(build)
    }
}

// Groups

async fn groups(client: &mut Client) -> Result<BTreeMap<String, Group>> {
    client
        .send_request(Request::Group(GroupRequest::List))
        .await?;
    match client.receive_response().await? {
        Response::Group(rsp) => Ok(rsp.groups),
        _ => unreachable!(),
    }
}

async fn add_group(client: &mut Client, group: &str) -> Result<()> {
    client
        .send_request(Request::Group(GroupRequest::Add {
            name: group.to_string(),
            parallel_tasks: Some(0), // Unlimited
        }))
        .await?;
    let response = client.receive_response().await?;
    if !response.success() {
        return Err(Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
            format!("{:?}", response),
        ))));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[tokio::test]
    async fn group_init() {
        let project_id = format!("pueue-test-{}", Utc::now().timestamp_millis());

        let p = Pueue::new(&project_id).await.unwrap(); // Group does not exist, create it
        let group_name = p.group;
        let mut client = Pueue::client().await.unwrap();
        let grps = groups(&mut client).await.unwrap();
        assert!(grps.contains_key(&group_name));
        drop(client);

        let p = Pueue::new(&project_id).await.unwrap(); // Group already exists
        let group_name = p.group.clone();
        let mut client = Pueue::client().await.unwrap();
        let grps = groups(&mut client).await.unwrap();
        assert!(grps.contains_key(&group_name));
        drop(client);

        p.clean().await.unwrap();
        let mut client = Pueue::client().await.unwrap();
        let grps = groups(&mut client).await.unwrap();
        assert!(!grps.contains_key(&group_name));
        drop(client);
    }
}
