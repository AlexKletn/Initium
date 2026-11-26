use super::*;

#[test]
fn closes_once_and_restarts_the_outro_after_a_reopened_menu_closes() {
    let mut phase = OutroPhase::WaitingForManager;
    assert!(matches!(
        phase.menu_interaction(false),
        MenuInteraction::UserspaceLocked
    ));
    assert!(matches!(
        phase.menu_interaction(true),
        MenuInteraction::CloseForHandoff
    ));
    assert!(!phase.ready_to_start(MenuPosition::Returning));
    assert!(phase.ready_to_start(MenuPosition::Closed));

    for active in [OutroPhase::Playing, OutroPhase::Finished] {
        phase = active;
        assert!(phase.interrupt_for_menu(MenuPosition::Open));
        assert!(matches!(
            phase.menu_interaction(true),
            MenuInteraction::UserspaceLocked
        ));
        assert!(!phase.ready_to_start(MenuPosition::Open));
        assert!(phase.ready_to_start(MenuPosition::Closed));
    }
}
