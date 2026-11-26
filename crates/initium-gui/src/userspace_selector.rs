use std::cell::Cell;
use std::io;

use initium_config::UserspaceChoice;

use crate::DisplayConnection;
use crate::choice_layout::ChoiceLayout;
use crate::choice_panel::{ChoicePanel, bind_click};

pub struct UserspaceSelector<'display> {
    panel: ChoicePanel<'display>,
    pending: Box<Cell<Option<usize>>>,
    selected: usize,
}

impl<'display> UserspaceSelector<'display> {
    pub(crate) fn create(
        display: &'display DisplayConnection,
        choices: &[UserspaceChoice],
    ) -> io::Result<Option<Self>> {
        let mut pending = Box::new(Cell::new(None));
        let selection = std::ptr::from_mut(pending.as_mut());
        let panel = ChoicePanel::create(
            display,
            choices
                .iter()
                .map(|choice| (choice.name.as_str(), choice.icon.as_path())),
            ChoiceLayout::Userspace,
            |tile| {
                // SAFETY: The selector owns the callback state until after the panel is dropped.
                unsafe { bind_click(tile, selection) }
            },
        )?;
        let Some(panel) = panel else { return Ok(None) };

        panel.select(0)?;
        Ok(Some(Self {
            panel,
            pending,
            selected: 0,
        }))
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn set_selected(&mut self, index: usize) -> io::Result<()> {
        if index != self.selected {
            self.panel.select(index)?;
            self.selected = index;
        }
        Ok(())
    }

    pub fn update(&mut self) -> io::Result<()> {
        if let Some(index) = self.pending.take() {
            self.set_selected(index)?;
        }
        Ok(())
    }
}
