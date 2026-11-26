use initium_config::Config;

use super::*;
use crate::CommandStep;

#[test]
fn validates_all_steps_before_execution() {
    let config = Config::parse(
        r#"
        [[steps]]
        id = "first"
        kind = "command"
        program = "/bin/true"

        [[steps]]
        id = "second"
        kind = "command"
        "#,
    )
    .expect("parse scenario");

    let mut flow = Flow::default();
    flow.register("command", CommandStep);

    let result = flow.run(&config.steps, |_| panic!("invalid scenario must not start"));
    assert!(matches!(result, Err(FlowError::InvalidStep { id, .. }) if id == "second"));
}

#[test]
fn reports_events_and_stops_on_failure() {
    let config = Config::parse(
        r#"
        [[steps]]
        id = "first"
        kind = "command"
        program = "/bin/true"

        [[steps]]
        id = "second"
        kind = "command"
        program = "/bin/false"

        [[steps]]
        id = "third"
        kind = "command"
        program = "/bin/true"
        "#,
    )
    .expect("parse scenario");

    let mut flow = Flow::default();
    flow.register("command", CommandStep);
    let mut events = Vec::new();
    let result = flow.run(&config.steps, |event| {
        let (state, id) = match event {
            StepEvent::Started { id } => ("started", id),
            StepEvent::Succeeded { id } => ("succeeded", id),
            StepEvent::Failed { id, .. } => ("failed", id),
        };
        events.push((state, id.to_owned()));
    });

    assert!(matches!(result, Err(FlowError::FailedStep { id, .. }) if id == "second"));
    assert_eq!(
        events
            .iter()
            .map(|(state, id)| (*state, id.as_str()))
            .collect::<Vec<_>>(),
        [
            ("started", "first"),
            ("succeeded", "first"),
            ("started", "second"),
            ("failed", "second"),
        ]
    );
}
