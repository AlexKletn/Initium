use initium_config::Config;

use super::*;

#[test]
fn rejects_relative_targets_during_preflight() {
    let config = Config::parse(
        r#"
        [[steps]]
        id = "mount-proc"
        kind = "mount"
        target = "proc"
        filesystem = "proc"
        "#,
    )
    .expect("parse mount configuration");

    assert!(matches!(
        MountStep.prepare(&config.steps[0]),
        Err(error) if error.kind() == io::ErrorKind::InvalidInput
    ));
}

#[test]
fn preserves_kernel_error_when_target_is_missing() {
    let config = Config::parse(
        r#"
        [[steps]]
        id = "mount-proc"
        kind = "mount"
        source = "proc"
        target = "/dev/null/target"
        filesystem = "proc"
        flags = ["no-suid", "no-dev", "no-exec", "read-only"]
        "#,
    )
    .expect("parse mount configuration");

    let operation = MountStep.prepare(&config.steps[0]).expect("prepare mount");
    let error = operation().expect_err("the target cannot exist");
    assert!(matches!(
        error.raw_os_error(),
        Some(libc::ENOTDIR | libc::EPERM | libc::EACCES)
    ));
}
