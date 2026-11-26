use std::io;
use std::time::{Duration, Instant};

use initium_config::UiConfig;
use initium_gui::{BootMenu, BootloaderImage, DisplayConnection, SplashAnimation, Touch};

use super::{menu_interaction::MenuInteraction, menu_position::MenuPosition};

const TOUCH_RETRY: Duration = Duration::from_millis(250);

pub(super) struct MenuInput<'display, 'config> {
    display: &'display DisplayConnection,
    config: &'config UiConfig,
    touch: Option<Touch<'display>>,
    menu: Option<BootMenu<'display, 'config>>,
    next_touch: Instant,
    offset: i32,
}

impl<'display, 'config> MenuInput<'display, 'config> {
    pub fn new(display: &'display DisplayConnection, config: &'config UiConfig) -> Self {
        Self {
            display,
            config,
            touch: None,
            menu: None,
            next_touch: Instant::now(),
            offset: 0,
        }
    }

    pub fn update(
        &mut self,
        now: Instant,
        interaction: MenuInteraction,
        image: &mut BootloaderImage<'_, '_>,
        animation: Option<&mut SplashAnimation<'_>>,
    ) -> io::Result<MenuPosition> {
        if let Some(path) = &self.config.touch_path
            && self.touch.is_none()
            && now >= self.next_touch
        {
            self.next_touch = now + TOUCH_RETRY;
            if let Ok(input) = Touch::open(self.display, path) {
                self.menu = Some(BootMenu::new(self.display, self.config)?);
                self.touch = Some(input);
                eprintln!("initium-gui-preview: boot menu ready");
            }
        }

        let Some(menu) = &mut self.menu else {
            return Ok(MenuPosition::Closed);
        };
        if matches!(interaction, MenuInteraction::CloseForHandoff) {
            menu.close_for_handoff(now)?;
        }
        if !matches!(interaction, MenuInteraction::Interactive) && menu.lock_userspace() {
            eprintln!("initium-gui-preview: userspace selector locked for root handoff");
        }
        let position = match menu.update(now) {
            Ok(position) => position,
            Err(error) => {
                eprintln!("initium-gui-preview: {error}");
                return Ok(MenuPosition::Returning);
            }
        };
        if position != self.offset {
            image.set_offset(position).map_err(io::Error::other)?;
            if let Some(animation) = animation {
                animation.set_offset(position).map_err(io::Error::other)?;
            }
            self.offset = position;
        }
        Ok(if menu.is_open() {
            MenuPosition::Open
        } else if menu.is_closed(now) {
            MenuPosition::Closed
        } else {
            MenuPosition::Returning
        })
    }
}
