use std::time::Duration;

use common::setup;
use jocker_lib::{
    common::{Exec as _, ProcessState},
    logs::{Logs, LogsArgs},
    ps::{Ps, PsArgs},
    start::{Start, StartArgs},
    stop::{Stop, StopArgs},
};
use tokio::time::sleep;

mod common;

#[tokio::test]
async fn start_log_stop_default() {
    let (state, tempdir) = setup().await;

    Start::new(StartArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;
    let ps_running_output = Ps::new(PsArgs::default(), state.clone()).exec().await;

    let logs = Logs::new(LogsArgs::default(), state.clone()).run().await;

    Stop::new(StopArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_running_output = ps_running_output.unwrap();
    let ps_stopped_output = Ps::new(PsArgs::default(), state.clone()).run().unwrap();

    assert_eq!(&ps_running_output[0].name, "eris");
    assert_eq!(&ps_running_output[0].status, &ProcessState::Running);
    assert_eq!(&ps_running_output[1].name, "harmonia");
    assert_eq!(&ps_running_output[1].status, &ProcessState::Running);
    assert_eq!(ps_running_output.len(), 2);

    assert_eq!(&ps_stopped_output[0].name, "eris");
    assert_eq!(&ps_stopped_output[0].status, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[1].name, "harmonia");
    assert_eq!(&ps_stopped_output[1].status, &ProcessState::Stopped);
    assert_eq!(ps_stopped_output.len(), 2);

    let (mut handles, mut rx) = logs.unwrap();
    let mut logs = Vec::new();

    while (handles.join_next().await).is_some() {}
    while let Some(message) = rx.recv().await {
        logs.push(message);
    }

    assert!(logs.len() >= 2);

    drop(tempdir);
}

#[tokio::test]
async fn start_log_stop_process_stack() {
    let (state, tempdir) = setup().await;
    state.set_current_stack(&Some("full".to_string())).unwrap();

    Start::new(StartArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;
    let ps_running_output = Ps::new(PsArgs::default(), state.clone()).exec().await;

    let logs = Logs::new(LogsArgs::default(), state.clone()).run().await;

    Stop::new(StopArgs::default(), state.clone())
        .exec()
        .await
        .unwrap();

    let ps_running_output = ps_running_output.unwrap();
    let ps_stopped_output = Ps::new(PsArgs::default(), state.clone()).run().unwrap();

    dbg!(&ps_running_output);
    assert_eq!(&ps_running_output[0].name, "ares");
    assert_eq!(&ps_running_output[0].status, &ProcessState::Running);
    assert_eq!(&ps_running_output[1].name, "athena");
    assert_eq!(&ps_running_output[1].status, &ProcessState::Running);
    assert_eq!(&ps_running_output[2].name, "eris");
    assert_eq!(&ps_running_output[2].status, &ProcessState::Running);
    assert_eq!(&ps_running_output[3].name, "harmonia");
    assert_eq!(&ps_running_output[3].status, &ProcessState::Running);
    assert_eq!(ps_running_output.len(), 4);

    assert_eq!(&ps_stopped_output[0].name, "ares");
    assert_eq!(&ps_stopped_output[0].status, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[1].name, "athena");
    assert_eq!(&ps_stopped_output[1].status, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[2].name, "eris");
    assert_eq!(&ps_stopped_output[2].status, &ProcessState::Stopped);
    assert_eq!(&ps_stopped_output[3].name, "harmonia");
    assert_eq!(&ps_stopped_output[3].status, &ProcessState::Stopped);
    assert_eq!(ps_stopped_output.len(), 4);

    let (mut handles, mut rx) = logs.unwrap();
    let mut logs = Vec::new();

    while (handles.join_next().await).is_some() {}
    while let Some(message) = rx.recv().await {
        logs.push(message);
    }

    assert!(logs.len() >= 4);

    drop(tempdir);
}

#[tokio::test]
async fn start_log_stop_process_stack_filter() {
    let (state, tempdir) = setup().await;
    state.set_current_stack(&Some("full".to_string())).unwrap();
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

    sleep(Duration::from_millis(100)).await;
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
    .unwrap();

    dbg!(&ps_running_output);
    assert_eq!(&ps_running_output[0].name, "athena");
    assert_eq!(&ps_running_output[0].status, &ProcessState::Running);
    assert_eq!(ps_running_output.len(), 1);

    assert_eq!(&ps_stopped_output[0].name, "athena");
    assert_eq!(&ps_stopped_output[0].status, &ProcessState::Stopped);
    assert_eq!(ps_stopped_output.len(), 1);

    let (mut handles, mut rx) = logs.unwrap();
    let mut logs = Vec::new();

    while (handles.join_next().await).is_some() {}
    while let Some(message) = rx.recv().await {
        logs.push(message);
    }

    assert!(!logs.is_empty());

    drop(tempdir);
}
