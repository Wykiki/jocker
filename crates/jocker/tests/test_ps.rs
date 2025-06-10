use common::{clean, setup};
use jocker_lib::ps::{Ps, PsArgs};

mod common;

#[tokio::test]
async fn ps_default() {
    let (state, tempdir) = setup().await;

    let ps_output = Ps::new(PsArgs::default(), state.clone())
        .run()
        .await
        .unwrap();

    assert_eq!(ps_output.iter().filter(|i| &i.name == "eris").count(), 1);
    assert_eq!(
        ps_output.iter().filter(|i| &i.name == "harmonia").count(),
        1
    );
    assert_eq!(ps_output.len(), 2);

    clean(state, tempdir).await.unwrap();
}

#[tokio::test]
async fn ps_default_with_stack() {
    let (state, tempdir) = setup().await;
    state
        .set_current_stack(&Some("full".to_owned()))
        .await
        .unwrap();

    let ps_output = Ps::new(PsArgs::default(), state.clone())
        .run()
        .await
        .unwrap();

    assert_eq!(ps_output.iter().filter(|i| &i.name == "eris").count(), 1);
    assert_eq!(
        ps_output.iter().filter(|i| &i.name == "harmonia").count(),
        1
    );
    assert_eq!(ps_output.iter().filter(|i| &i.name == "ares").count(), 1);
    assert_eq!(ps_output.iter().filter(|i| &i.name == "athena").count(), 1);
    assert_eq!(ps_output.len(), 4);

    clean(state, tempdir).await.unwrap();
}

#[tokio::test]
async fn ps_filter() {
    let (state, tempdir) = setup().await;

    let ps_output = Ps::new(
        PsArgs {
            processes: vec!["eris".to_owned()],
        },
        state.clone(),
    )
    .run()
    .await
    .unwrap();

    assert_eq!(ps_output.iter().filter(|i| &i.name == "eris").count(), 1);
    assert_eq!(ps_output.len(), 1);

    clean(state, tempdir).await.unwrap();
}

#[tokio::test]
async fn ps_filter_with_stack() {
    let (state, tempdir) = setup().await;
    state
        .set_current_stack(&Some("full".to_owned()))
        .await
        .unwrap();

    let ps_output = Ps::new(
        PsArgs {
            processes: vec!["eris".to_owned(), "athena".to_owned()],
        },
        state.clone(),
    )
    .run()
    .await
    .unwrap();

    assert_eq!(ps_output.iter().filter(|i| &i.name == "eris").count(), 1);
    assert_eq!(ps_output.iter().filter(|i| &i.name == "athena").count(), 1);
    assert_eq!(ps_output.len(), 2);

    clean(state, tempdir).await.unwrap();
}
