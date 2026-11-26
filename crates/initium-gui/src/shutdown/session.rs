use std::process::Command;

pub(super) fn graphical_session_exists(seat_name: &str) -> bool {
    let Ok(seat) = Command::new("/usr/bin/loginctl")
        .args(["show-seat", seat_name, "--property=Sessions", "--value"])
        .output()
    else {
        return false;
    };
    seat.status.success()
        && String::from_utf8_lossy(&seat.stdout)
            .split_whitespace()
            .any(|session| {
                Command::new("/usr/bin/loginctl")
                    .args(["show-session", session, "--property=Type", "--value"])
                    .output()
                    .is_ok_and(|result| {
                        result.status.success()
                            && matches!(
                                String::from_utf8_lossy(&result.stdout).trim(),
                                "wayland" | "x11" | "mir"
                            )
                    })
            })
}
