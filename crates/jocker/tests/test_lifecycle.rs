use common::{clean, setup};
use jocker_lib::{
    common::{Exec as _, ProcessState},
    logs::{Logs, LogsArgs},
    ps::{Ps, PsArgs},
    start::{Start, StartArgs},
    stop::{Stop, StopArgs},
};
use pueue_lib::{Client, Request, Response, Settings};

mod common;

#[tokio::test]
async fn start_log_stop_default() {
    let (state, tempdir) = setup().await;

    Start::new(StartArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_running_output = Ps::new(PsArgs::default(), state.clone()).exec().await;

    let logs = Logs::new(LogsArgs::default(), state.clone()).run().await;

    Stop::new(StopArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_running_output = ps_running_output.unwrap();
    let ps_stopped_output = Ps::new(PsArgs::default(), state.clone())
        .run()
        .await
        .unwrap();

    assert_eq!(&ps_running_output[0].name, "eris");
    assert_eq!(&ps_running_output[0].state, &ProcessState::Running);
    assert_eq!(&ps_running_output[1].name, "harmonia");
    assert_eq!(&ps_running_output[1].state, &ProcessState::Running);
    assert_eq!(ps_running_output.len(), 2);

    assert_eq!(&ps_stopped_output[0].name, "eris");
    assert_eq!(&ps_stopped_output[0].state, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[1].name, "harmonia");
    assert_eq!(&ps_stopped_output[1].state, &ProcessState::Stopped);
    assert_eq!(ps_stopped_output.len(), 2);

    let (mut handles, mut rx) = logs.unwrap();
    let mut logs = Vec::new();

    while (handles.join_next().await).is_some() {}
    while let Some(message) = rx.recv().await {
        logs.push(message);
    }

    assert!(logs.len() >= 2);

    clean(state, tempdir).await.unwrap();
}

#[tokio::test]
async fn start_log_stop_process_stack() {
    let (state, tempdir) = setup().await;
    state
        .set_current_stack(&Some("full".to_string()))
        .await
        .unwrap();

    Start::new(StartArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_running_output = Ps::new(PsArgs::default(), state.clone()).exec().await;

    let logs = Logs::new(LogsArgs::default(), state.clone()).run().await;

    Stop::new(StopArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_running_output = ps_running_output.unwrap();
    let ps_stopped_output = Ps::new(PsArgs::default(), state.clone())
        .run()
        .await
        .unwrap();

    assert_eq!(&ps_running_output[0].name, "ares");
    assert_eq!(&ps_running_output[0].state, &ProcessState::Running);
    assert_eq!(&ps_running_output[1].name, "athena");
    assert_eq!(&ps_running_output[1].state, &ProcessState::Running);
    assert_eq!(&ps_running_output[2].name, "eris");
    assert_eq!(&ps_running_output[2].state, &ProcessState::Running);
    assert_eq!(&ps_running_output[3].name, "harmonia");
    assert_eq!(&ps_running_output[3].state, &ProcessState::Running);
    assert_eq!(ps_running_output.len(), 4);

    assert_eq!(&ps_stopped_output[0].name, "ares");
    assert_eq!(&ps_stopped_output[0].state, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[1].name, "athena");
    assert_eq!(&ps_stopped_output[1].state, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[2].name, "eris");
    assert_eq!(&ps_stopped_output[2].state, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[3].name, "harmonia");
    assert_eq!(&ps_stopped_output[3].state, &ProcessState::Stopped);
    assert_eq!(ps_stopped_output.len(), 4);

    let (mut handles, mut rx) = logs.unwrap();
    let mut logs = Vec::new();

    while (handles.join_next().await).is_some() {}
    while let Some(message) = rx.recv().await {
        logs.push(message);
    }

    assert!(logs.len() >= 4);

    clean(state, tempdir).await.unwrap();
}

#[tokio::test]
async fn start_log_stop_process_stack_filter() {
    let (state, tempdir) = setup().await;
    state
        .set_current_stack(&Some("full".to_string()))
        .await
        .unwrap();
    let processes = vec!["athena".to_owned()];

    Start::new(
        StartArgs {
            processes: processes.clone(),
        },
        state.clone(),
    )
    .exec()
    .await
    .unwrap();

    let ps_running_output = Ps::new(
        PsArgs {
            processes: processes.clone(),
        },
        state.clone(),
    )
    .exec()
    .await;

    let logs = Logs::new(LogsArgs::default(), state.clone()).run().await;

    Stop::new(
        StopArgs {
            processes: processes.clone(),
            ..Default::default()
        },
        state.clone(),
    )
    .exec()
    .await
    .unwrap();

    let ps_running_output = ps_running_output.unwrap();
    let ps_stopped_output = Ps::new(
        PsArgs {
            processes: processes.clone(),
        },
        state.clone(),
    )
    .run()
    .await
    .unwrap();

    assert_eq!(&ps_running_output[0].name, "athena");
    assert_eq!(&ps_running_output[0].state, &ProcessState::Running);
    assert_eq!(ps_running_output.len(), 1);

    assert_eq!(&ps_stopped_output[0].name, "athena");
    assert_eq!(&ps_stopped_output[0].state, &ProcessState::Stopped);
    assert_eq!(ps_stopped_output.len(), 1);

    let (mut handles, mut rx) = logs.unwrap();
    let mut logs = Vec::new();

    while (handles.join_next().await).is_some() {}
    while let Some(message) = rx.recv().await {
        logs.push(message);
    }

    assert!(!logs.is_empty());

    clean(state, tempdir).await.unwrap();
}

#[tokio::test]
async fn start_after_stop() {
    let (state, tempdir) = setup().await;

    Start::new(StartArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_run_1 = Ps::new(PsArgs::default(), state.clone()).exec().await;

    Stop::new(StopArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    Start::new(StartArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    Stop::new(StopArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_run_1 = ps_run_1.unwrap();
    let ps_run_2 = Ps::new(PsArgs::default(), state.clone())
        .run()
        .await
        .unwrap();

    assert_eq!(&ps_run_1[0].name, "eris");
    assert_eq!(&ps_run_1[0].state, &ProcessState::Running);
    assert_eq!(&ps_run_1[1].name, "harmonia");
    assert_eq!(&ps_run_1[1].state, &ProcessState::Running);
    assert_eq!(ps_run_1.len(), 2);

    assert_eq!(&ps_run_2[0].name, "eris");
    assert_eq!(&ps_run_2[0].state, &ProcessState::Stopped);
    assert_eq!(&ps_run_2[1].name, "harmonia");
    assert_eq!(&ps_run_2[1].state, &ProcessState::Stopped);
    assert_eq!(ps_run_2.len(), 2);

    let (settings, _) = Settings::read(&None).unwrap();
    let mut client = Client::new(settings, true).await.unwrap();
    client.send_request(Request::Status).await.unwrap();
    if let Response::Status(status) = client.receive_response().await.unwrap() {
        assert_eq!(status.task_ids_in_group(state.scheduler_group()).len(), 2);
    } else {
        panic!();
    };

    clean(state, tempdir).await.unwrap();
}
