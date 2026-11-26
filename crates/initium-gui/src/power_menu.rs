use std::cell::Cell;
use std::io;
use std::process::{Child, Command};

use initium_config::{PowerCommand, PowerMenuItem};

use crate::DisplayConnection;
use crate::{
    choice_layout::ChoiceLayout,
    choice_panel::{ChoicePanel, bind_click},
};

pub struct PowerMenu<'display, 'config> {
    _panel: ChoicePanel<'display>,
    selected: Box<Cell<Option<usize>>>,
    items: &'config [PowerMenuItem],
}

impl<'display, 'config> PowerMenu<'display, 'config> {
    pub(crate) fn create(
        display: &'display DisplayConnection,
        items: &'config [PowerMenuItem],
    ) -> io::Result<Option<Self>> {
        let mut selected = Box::new(Cell::new(None));
        let selection = std::ptr::from_mut(selected.as_mut());
        let panel = ChoicePanel::create(
            display,
            items
                .iter()
                .map(|item| (item.name.as_str(), item.icon.as_path())),
            ChoiceLayout::Power,
            |tile| {
                // SAFETY: The callback data remains live until the panel is dropped.
                unsafe { bind_click(tile, selection) }
            },
        )?;

        Ok(panel.map(|panel| Self {
            _panel: panel,
            selected,
            items,
        }))
    }

    pub fn take_command(&self) -> Option<&PowerCommand> {
        self.selected
            .take()
            .and_then(|index| self.items.get(index))
            .map(|item| &item.command)
    }

    pub fn spawn_pending_command(&self) -> io::Result<Option<Child>> {
        self.take_command()
            .map(|command| Command::new(&command.program).args(&command.args).spawn())
            .transpose()
    }
}
