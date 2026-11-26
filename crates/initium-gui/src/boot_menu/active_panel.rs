use super::action::Action;
use crate::{BootLogPanel, HardwarePanel, PowerMenu};

pub(super) enum ActivePanel<'display, 'config> {
    Journal { _panel: BootLogPanel<'display> },
    Hardware(HardwarePanel<'display, 'config>),
    Power(PowerMenu<'display, 'config>),
}

impl ActivePanel<'_, '_> {
    pub fn kind(&self) -> Action {
        match self {
            Self::Journal { .. } => Action::Journal,
            Self::Hardware(_) => Action::Hardware,
            Self::Power(_) => Action::Power,
        }
    }
}
