#[path = "initium-gui-preview/greeter.rs"]
mod greeter;
#[path = "initium-gui-preview/handoff.rs"]
mod handoff;
#[path = "initium-gui-preview/menu_input.rs"]
mod menu_input;
#[path = "initium-gui-preview/menu_interaction.rs"]
mod menu_interaction;
#[path = "initium-gui-preview/menu_position.rs"]
mod menu_position;
#[path = "initium-gui-preview/outro_phase.rs"]
mod outro_phase;
#[path = "initium-gui-preview/runtime.rs"]
mod runtime;

use std::error::Error;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use initium_config::{Config, DisplayConfig, Theme};
use initium_gui::{BootloaderFrame, DisplayPort, DrmKms, connect_display, initialize};

const HOLD_TIME: Duration = Duration::from_secs(5);

fn main() -> ExitCode {
    match preview() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("initium-gui-preview: {error}");
            ExitCode::FAILURE
        }
    }
}

fn preview() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let config_path = args.next().ok_or_else(usage)?;
    let hold = match args.next() {
        None => false,
        Some(flag) if flag == "--hold" && args.next().is_none() => true,
        Some(_) => return Err(usage().into()),
    };

    let theme = Theme::load_system()?;
    let config = Config::parse(&std::fs::read_to_string(config_path)?)?;
    let display = config
        .ui
        .display
        .as_ref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing [ui.display]"))?;
    let bootloader_frame = config.ui.splash.bootloader_frame.as_ref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing [ui.splash.bootloader_frame]",
        )
    })?;

    let default_framebuffer = match display {
        DisplayConfig::Fbdev { path } => path.as_path(),
        DisplayConfig::DrmKms { .. } => Path::new("/dev/fb0"),
    };
    let framebuffer = bootloader_frame
        .capture_device
        .as_deref()
        .unwrap_or(default_framebuffer);
    let frame = BootloaderFrame::capture(framebuffer, bootloader_frame)
        .map_err(|source| io::Error::other(format!("capture bootloader frame: {source}")))?;
    let (width, height) = frame.size();
    eprintln!("initium-gui-preview: captured {width}x{height} bootloader frame");

    let drm = match display {
        DisplayConfig::DrmKms { path, connector_id } => {
            Some(DrmKms::new(path, *connector_id).resolve_connector()?)
        }
        DisplayConfig::Fbdev { .. } => None,
    };
    let previous_scanout = drm
        .as_ref()
        .map(DrmKms::scanout_framebuffer)
        .transpose()?
        .flatten();
    initialize()?;
    let display = match &drm {
        Some(drm) => drm.connect(),
        None => connect_display(display),
    }
    .map_err(|source| io::Error::other(format!("connect display: {source}")))?
    .with_theme(theme);
    let mut image = display.show_bootloader_frame(&frame)?;
    if let Some(brand) = &config.ui.splash.brand_text {
        image.show_brand(brand)?;
    }
    display.refresh()?;
    if let Some(drm) = &drm {
        let framebuffer = wait_for_scanout(drm, previous_scanout)?;
        eprintln!("initium-gui-preview: DRM framebuffer {framebuffer} active");
    } else {
        eprintln!("initium-gui-preview: first frame rendered");
    }
    let deadline = (!hold).then(|| Instant::now() + HOLD_TIME);
    runtime::run(&display, drm.as_ref(), &mut image, &config.ui, deadline)?;
    Ok(())
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: initium-gui-preview <config.toml> [--hold]",
    )
}

fn wait_for_scanout(drm: &DrmKms, previous: Option<u32>) -> io::Result<u32> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(framebuffer) = drm.scanout_framebuffer()?
            && Some(framebuffer) != previous
        {
            return Ok(framebuffer);
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "DRM scanout did not change",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
