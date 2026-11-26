use std::fs;

use super::*;

#[test]
fn locks_only_after_the_last_boot_pause_and_never_reopens() {
    let directory = std::env::temp_dir().join(format!("initium-handoff-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("create boot state");
    let stage = directory.join("stage");
    let pause = directory.join("boot-pause");
    fs::write(&stage, "modules-ready\n").expect("write initial stage");

    let mut handoff = Handoff::new(Some(&directory));
    let now = Instant::now();
    handoff.poll(now);
    assert!(!handoff.locked());
    assert!(!handoff.switching_root());
    fs::write(&pause, "on\n").expect("enable boot pause");
    fs::write(&stage, "root-ready\n").expect("prepare system root");
    handoff.poll(now + CHECK_INTERVAL);
    assert!(!handoff.locked());

    fs::remove_file(pause).expect("close boot pause");
    handoff.poll(now + CHECK_INTERVAL * 2);
    assert!(handoff.locked());
    assert!(!handoff.switching_root());
    fs::write(&stage, "switching-root\n").expect("start root handoff");
    handoff.poll(now + CHECK_INTERVAL * 3);
    assert!(handoff.switching_root());
    fs::write(&stage, "root-failed\n").expect("record handoff failure");
    handoff.poll(now + CHECK_INTERVAL * 4);
    assert!(handoff.locked());
    fs::remove_dir_all(directory).expect("remove fixture");
}

#[test]
fn recognizes_handoff_if_the_ready_stage_was_skipped() {
    assert!(switch_root_started("switching-root"));
    assert!(switch_root_started("userspace"));
    assert!(!switch_root_started("modules-ready"));
}
