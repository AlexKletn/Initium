use std::io;
use std::time::{Duration, Instant};

use initium_config::{ShutdownConfig, Theme};

use super::{ShutdownAction, caption::Caption, frame::Frame, session::graphical_session_exists};
use crate::{DisplayPort, Fbdev};

pub fn show_shutdown(
    config: &ShutdownConfig,
    action: ShutdownAction,
    theme: Theme,
) -> io::Result<()> {
    let custom = match action {
        ShutdownAction::Poweroff => config.poweroff_caption.as_deref(),
        ShutdownAction::Reboot => config.reboot_caption.as_deref(),
    };
    let caption = Caption::load(action, custom, config.caption_size, theme)?;
    let connector_id = config
        .connector_id_file
        .as_ref()
        .map(|path| {
            std::fs::read_to_string(path)?
                .trim()
                .parse::<u32>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .transpose()?;

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline
        && graphical_session_exists(config.seat.as_deref().unwrap_or("seat0"))
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    let frame = loop {
        match Frame::present(config, connector_id, &caption) {
            Ok(frame) => break frame,
            Err(error)
                if error.raw_os_error() == Some(libc::EBUSY) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error),
        }
    };
    eprintln!("initium-shutdown: caption presented on DRM");

    let console_ready = if let Some(path) = &config.console_framebuffer {
        match Fbdev::new(path)
            .connect()
            .and_then(|display| caption.render_on(&display))
        {
            Ok(()) => true,
            Err(error) => {
                eprintln!("initium-shutdown: could not mirror caption to fbcon: {error}");
                false
            }
        }
    } else {
        false
    };

    frame.leave_on_screen()?;
    if console_ready && let Some(path) = &config.console_blank {
        std::fs::write(path, b"0\n")?;
    }
    Ok(())
}
