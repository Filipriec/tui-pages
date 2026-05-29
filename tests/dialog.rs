#![cfg(feature = "dialog")]

use tui_pages::dialog::{self, DialogData, DialogResult};
use tui_pages::{FocusController, FocusIntent, FocusManager, FocusTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Purpose {
    ConfirmDelete,
}

type Focus = FocusManager<DialogData<Purpose>, ()>;

#[test]
fn dialog_shows_navigates_and_resolves() {
    let mut focus: Focus = FocusManager::new();
    focus.register_page(vec![FocusTarget::Button(0)]);

    // Show a two-button dialog via the focus intent the dialog builds.
    let data = DialogData::new(
        "Delete?",
        "This cannot be undone.",
        ["Delete", "Cancel"],
        Purpose::ConfirmDelete,
    );
    focus.apply_focus_intent(data.show_intent());

    // The overlay is the dialog; focus reports the active button.
    assert_eq!(focus.current(), Some(FocusTarget::DialogButton(0)));
    assert_eq!(dialog::active_button(&focus), Some(0));
    assert_eq!(
        dialog::current_dialog(&focus).map(|d| d.button_count()),
        Some(2)
    );

    // Next moves between buttons (clamped, no wrap).
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(dialog::active_button(&focus), Some(1));
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(dialog::active_button(&focus), Some(1));

    // Resolve the selection, then close.
    let result = dialog::selection(&focus);
    assert_eq!(
        result,
        Some(DialogResult::Selected {
            purpose: Some(Purpose::ConfirmDelete),
            index: 1,
        })
    );

    focus.apply_focus_intent(FocusIntent::ClearOverlay);
    assert!(dialog::current_dialog(&focus).is_none());
    assert_eq!(focus.current(), Some(FocusTarget::Button(0)));
}

#[test]
fn loading_dialog_has_no_buttons() {
    let mut focus: Focus = FocusManager::new();
    let data = DialogData::loading("Please wait", "Saving…");
    focus.apply_focus_intent(data.show_intent());

    assert_eq!(
        dialog::current_dialog(&focus).map(|d| d.button_count()),
        Some(0)
    );
}
