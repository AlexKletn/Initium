#[derive(Clone, Copy)]
pub(super) enum MenuInteraction {
    Interactive,
    UserspaceLocked,
    CloseForHandoff,
}
