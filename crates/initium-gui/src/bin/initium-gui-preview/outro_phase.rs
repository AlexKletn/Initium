use super::{menu_interaction::MenuInteraction, menu_position::MenuPosition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutroPhase {
    WaitingForManager,
    WaitingForMenu,
    Playing,
    Finished,
    Disabled,
}

impl OutroPhase {
    pub fn menu_interaction(&mut self, manager_started: bool) -> MenuInteraction {
        if *self == Self::WaitingForManager && manager_started {
            *self = Self::WaitingForMenu;
            MenuInteraction::CloseForHandoff
        } else {
            MenuInteraction::UserspaceLocked
        }
    }

    pub fn ready_to_start(self, menu: MenuPosition) -> bool {
        self == Self::WaitingForMenu && menu == MenuPosition::Closed
    }

    pub fn interrupt_for_menu(&mut self, menu: MenuPosition) -> bool {
        if matches!(self, Self::Playing | Self::Finished) && menu == MenuPosition::Open {
            *self = Self::WaitingForMenu;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
#[path = "outro_phase.tests.rs"]
mod tests;
