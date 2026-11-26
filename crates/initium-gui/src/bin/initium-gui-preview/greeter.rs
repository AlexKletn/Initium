use std::io;
use std::time::{Duration, Instant};

use initium_gui::{DisplayConnection, DrmKms, SplashAnimation};

use crate::{
    handoff::Handoff, menu_interaction::MenuInteraction, menu_position::MenuPosition,
    outro_phase::OutroPhase,
};

const CHECK_INTERVAL: Duration = Duration::from_millis(8);

pub(super) fn wait(
    display: &DisplayConnection,
    drm: Option<&DrmKms>,
    handoff: &mut Handoff<'_>,
    mut animation: Option<&mut SplashAnimation<'_>>,
    deadline: Option<Instant>,
    mut update_menu: impl FnMut(
        Instant,
        MenuInteraction,
        Option<&mut SplashAnimation<'_>>,
    ) -> io::Result<MenuPosition>,
) -> io::Result<()> {
    if drm.is_none() {
        return Ok(());
    }

    display.refresh().map_err(io::Error::other)?;
    if let Err(error) = display.wait_for_drm_vblank() {
        eprintln!("initium-gui-preview: could not wait for DRM vblank: {error}");
    }
    let previous = display.drm_scanout()?;
    let can_draw =
        previous.is_some_and(
            |framebuffer| match display.draw_into_drm_front_buffer(framebuffer) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("initium-gui-preview: keeping static boot frame: {error}");
                    false
                }
            },
        );
    if !can_draw {
        animation = None;
    }

    display.release_drm_master()?;
    eprintln!("initium-gui-preview: DRM master released, keeping framebuffer {previous:?}");

    let mut next_frame = Instant::now();
    let mut outro = OutroPhase::WaitingForManager;
    let mut loop_finished = false;
    let mut reported_animation = false;
    let mut reported_publish_failure = false;
    while deadline.is_none_or(|deadline| Instant::now() < deadline) {
        let now = Instant::now();
        handoff.poll(now);
        let current = display.drm_scanout().unwrap_or_default();
        if handoff.greeter_presented() || replaced(previous, current) {
            eprintln!("initium-gui-preview: display manager replaced the boot framebuffer");
            return Ok(());
        }

        let mut menu = MenuPosition::Closed;
        if can_draw {
            let interaction = outro.menu_interaction(handoff.display_manager_started());
            menu = update_menu(now, interaction, animation.as_deref_mut())?;
            if outro.interrupt_for_menu(menu) {
                if let Some(animation) = &mut animation {
                    animation.cancel_outro()?;
                }
                next_frame = now;
            }
        }

        if let Some(animation) = &mut animation
            && now >= next_frame
            && current == previous
            && outro != OutroPhase::Finished
        {
            if outro.ready_to_start(menu) {
                outro = if animation.begin_outro(now) {
                    eprintln!("initium-gui-preview: final splash animation started");
                    OutroPhase::Playing
                } else {
                    OutroPhase::Disabled
                };
            }
            let frame = if outro == OutroPhase::Playing {
                let (interval, ongoing) = animation.advance_outro(now)?;
                if !ongoing {
                    outro = OutroPhase::Finished;
                    eprintln!("initium-gui-preview: final splash animation finished");
                }
                Some(interval)
            } else if loop_finished {
                None
            } else {
                animation.advance()?
            };
            if let Some(interval) = frame {
                next_frame = now + interval;
                if !reported_animation {
                    eprintln!("initium-gui-preview: animation continues without DRM master");
                    reported_animation = true;
                }
            } else {
                loop_finished = true;
                next_frame = now + CHECK_INTERVAL;
            }
        }
        if can_draw {
            display.poll_events().map_err(io::Error::other)?;
        }
        if let Some(framebuffer) = current.filter(|_| can_draw && current == previous)
            && let Err(error) = display.publish_drm_front_buffer(framebuffer)
            && !reported_publish_failure
        {
            eprintln!("initium-gui-preview: boot frame synchronization failed: {error}");
            reported_publish_failure = true;
        }
        let now = Instant::now();
        let check = now + CHECK_INTERVAL;
        let wake = if animation.is_some() && outro != OutroPhase::Finished && current == previous {
            next_frame.min(check)
        } else {
            check
        };
        std::thread::sleep(wake.saturating_duration_since(now));
    }
    Ok(())
}

fn replaced(previous: Option<u32>, current: Option<u32>) -> bool {
    current.is_some() && current != previous
}

#[cfg(test)]
#[path = "greeter.tests.rs"]
mod tests;
