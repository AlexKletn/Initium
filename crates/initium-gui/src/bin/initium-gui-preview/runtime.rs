use std::io;
use std::time::{Duration, Instant};

use initium_config::UiConfig;
use initium_gui::{BootloaderImage, DisplayConnection, DrmKms, SplashAnimation};

use crate::{greeter, handoff::Handoff, menu_input::MenuInput, menu_interaction::MenuInteraction};

const UI_TICK: Duration = Duration::from_millis(8);

pub fn run(
    display: &DisplayConnection,
    drm: Option<&DrmKms>,
    image: &mut BootloaderImage<'_, '_>,
    config: &UiConfig,
    deadline: Option<Instant>,
) -> io::Result<()> {
    let mut animation = SplashAnimation::open(display, &config.splash)?;
    let mut input = MenuInput::new(display, config);
    let mut handoff = Handoff::new(config.boot_state_dir.as_deref());
    let mut next_frame = animation.as_ref().map(|_| Instant::now());
    let mut first = true;

    loop {
        let now = Instant::now();
        if deadline.is_some_and(|deadline| now >= deadline) {
            return Ok(());
        }

        handoff.poll(now);
        let interaction = if handoff.locked() {
            MenuInteraction::UserspaceLocked
        } else {
            MenuInteraction::Interactive
        };
        input.update(now, interaction, image, animation.as_mut())?;
        if let Some(animation) = &mut animation
            && next_frame.is_some_and(|next| now >= next)
        {
            next_frame = animation.advance()?.map(|interval| now + interval);
            if first && next_frame.is_some() {
                image.hide_background().map_err(io::Error::other)?;
                display.refresh().map_err(io::Error::other)?;
                eprintln!("initium-gui-preview: first animation frame rendered");
                first = false;
            }
        }

        if handoff.switching_root() {
            return greeter::wait(
                display,
                drm,
                &mut handoff,
                animation.as_mut(),
                deadline,
                |now, interaction, animation| input.update(now, interaction, image, animation),
            );
        }

        display.poll_events().map_err(io::Error::other)?;
        let now = Instant::now();
        let wake = next_frame
            .into_iter()
            .chain(deadline)
            .fold(now + UI_TICK, Instant::min);
        std::thread::sleep(wake.saturating_duration_since(now));
    }
}
