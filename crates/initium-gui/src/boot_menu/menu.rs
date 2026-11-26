use std::io;
use std::process::Child;
use std::time::{Duration, Instant};

use initium_config::UiConfig;

use super::{
    action::Action, active_panel::ActivePanel, controls::Controls, lift::Lift, logo_tap::LogoTap,
};
use crate::{DisplayConnection, UserspaceSelector};

pub struct BootMenu<'display, 'config> {
    display: &'display DisplayConnection,
    config: &'config UiConfig,
    power_command: Option<Child>,
    active: Option<ActivePanel<'display, 'config>>,
    userspace: Option<UserspaceSelector<'display>>,
    selected_userspace: usize,
    userspace_locked: bool,
    controls: Controls<'display>,
    logo: LogoTap<'display>,
    open: bool,
    visible: bool,
    lift: Lift,
    last_logo_position: (i32, bool),
    next_hardware_update: Instant,
}

impl<'display, 'config> BootMenu<'display, 'config> {
    pub fn new(
        display: &'display DisplayConnection,
        config: &'config UiConfig,
    ) -> io::Result<Self> {
        let mut actions = vec![Action::Journal];
        if !config.hardware.is_empty() {
            actions.push(Action::Hardware);
        }
        if !config.power_menu.is_empty() {
            actions.push(Action::Power);
        }

        Ok(Self {
            display,
            config,
            active: None,
            power_command: None,
            userspace: None,
            selected_userspace: 0,
            userspace_locked: false,
            controls: Controls::create(display, actions)?,
            logo: LogoTap::create(display)?,
            open: false,
            visible: false,
            lift: Lift::default(),
            last_logo_position: (0, true),
            next_hardware_update: Instant::now(),
        })
    }

    pub fn update(&mut self, now: Instant) -> io::Result<i32> {
        if let Some(selector) = &mut self.userspace {
            selector.update()?;
            self.selected_userspace = selector.selected();
        }
        if self.logo.take() && self.lift.is_settled(now) {
            self.set_open(!self.open, now)?;
        }

        let position = self.lift.offset(now);
        let settled = self.lift.is_settled(now);
        if (position, settled) != self.last_logo_position {
            self.logo.position(position, settled)?;
            self.last_logo_position = (position, settled);
        }

        let visible = self.open && settled;
        if self.visible != visible {
            self.controls.show(visible)?;
            self.visible = visible;
            if visible && !self.userspace_locked {
                self.userspace = self
                    .display
                    .show_userspace_selector(&self.config.userspace)?;
                if let Some(selector) = &mut self.userspace {
                    selector.set_selected(self.selected_userspace)?;
                }
            }
        }

        if let Some(action) = self.controls.take()
            && self.visible
        {
            self.open_panel(action)?;
        }
        self.update_active(now)?;
        self.poll_power_command()?;
        Ok(position)
    }

    pub fn close_for_handoff(&mut self, now: Instant) -> io::Result<()> {
        self.set_open(false, now)
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_closed(&self, now: Instant) -> bool {
        !self.open && self.lift.is_settled(now)
    }

    fn set_open(&mut self, open: bool, now: Instant) -> io::Result<()> {
        if self.open == open {
            return Ok(());
        }
        self.open = open;
        self.lift.set_open(open, now);
        if !open {
            self.active = None;
            self.userspace = None;
            self.controls.select(None)?;
        }
        Ok(())
    }

    pub fn lock_userspace(&mut self) -> bool {
        if std::mem::replace(&mut self.userspace_locked, true) {
            return false;
        }
        self.userspace = None;
        true
    }

    fn update_active(&mut self, now: Instant) -> io::Result<()> {
        match &self.active {
            Some(ActivePanel::Hardware(panel)) if now >= self.next_hardware_update => {
                self.next_hardware_update = now + Duration::from_millis(250);
                panel.update()?;
            }
            Some(ActivePanel::Power(menu)) => {
                if self.power_command.is_none() {
                    self.power_command = menu.spawn_pending_command()?;
                } else {
                    menu.take_command();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn poll_power_command(&mut self) -> io::Result<()> {
        let Some(child) = &mut self.power_command else {
            return Ok(());
        };
        if let Some(status) = child.try_wait()? {
            self.power_command = None;
            if !status.success() {
                return Err(io::Error::other(format!(
                    "power command exited with {status}"
                )));
            }
        }
        Ok(())
    }

    fn open_panel(&mut self, action: Action) -> io::Result<()> {
        let previous = self.active.take();
        let closing = previous
            .as_ref()
            .is_some_and(|panel| panel.kind() == action);
        drop(previous);
        self.controls.select(None)?;
        if closing {
            return Ok(());
        }

        self.active = match action {
            Action::Journal => Some(ActivePanel::Journal {
                _panel: self.display.show_boot_log()?,
            }),
            Action::Hardware => self
                .display
                .show_hardware(&self.config.hardware)?
                .map(ActivePanel::Hardware),
            Action::Power => self
                .display
                .show_power_menu(&self.config.power_menu)?
                .map(ActivePanel::Power),
        };
        self.controls
            .select(self.active.as_ref().map(ActivePanel::kind))
    }
}
