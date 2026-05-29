#![cfg(feature = "dialog")]

use crossterm::event::{KeyCode, KeyEvent};
use tui_pages::dialog::{self, DialogData, DialogKey, DialogResult};
use tui_pages::{FocusController, FocusIntent, FocusManager, FocusTarget, FocusWrap};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::from(code)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Purpose {
    ConfirmDelete,
}

// O = () (no named overlays), M = DialogData<Purpose> (the modal payload).
type Focus = FocusManager<(), DialogData<Purpose>>;

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
    assert_eq!(focus.current(), Some(FocusTarget::ModalItem(0)));
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
fn handle_key_drives_the_conventional_bindings() {
    let mut focus: Focus = FocusManager::new();
    focus.register_page(vec![FocusTarget::Button(0)]);

    // With no dialog open, keys are left untouched for the rest of the loop.
    assert_eq!(dialog::handle_key(&mut focus, key(KeyCode::Enter)), DialogKey::Ignored);

    let data = DialogData::new("Delete?", "msg", ["Delete", "Cancel"], Purpose::ConfirmDelete);
    focus.apply_focus_intent(data.show_intent());

    // Tab/Right advance, BackTab/Left retreat — all consumed while modal.
    assert_eq!(dialog::handle_key(&mut focus, key(KeyCode::Tab)), DialogKey::Consumed);
    assert_eq!(dialog::active_button(&focus), Some(1));
    assert_eq!(dialog::handle_key(&mut focus, key(KeyCode::Left)), DialogKey::Consumed);
    assert_eq!(dialog::active_button(&focus), Some(0));
    // Unbound keys are swallowed by the modal rather than leaking through.
    assert_eq!(dialog::handle_key(&mut focus, key(KeyCode::Char('x'))), DialogKey::Consumed);

    // Enter resolves to the active button and closes the dialog.
    assert_eq!(
        dialog::handle_key(&mut focus, key(KeyCode::Enter)),
        DialogKey::Resolved(DialogResult::Selected {
            purpose: Some(Purpose::ConfirmDelete),
            index: 0,
        })
    );
    assert!(dialog::current_dialog(&focus).is_none());
    assert_eq!(focus.current(), Some(FocusTarget::Button(0)));

    // Esc dismisses and closes.
    focus.apply_focus_intent(
        DialogData::new("Delete?", "msg", ["Delete"], Purpose::ConfirmDelete).show_intent(),
    );
    assert_eq!(
        dialog::handle_key(&mut focus, key(KeyCode::Esc)),
        DialogKey::Resolved(DialogResult::Dismissed)
    );
    assert!(dialog::current_dialog(&focus).is_none());
}

#[test]
fn dialog_buttons_wrap_when_enabled() {
    let mut focus: Focus = FocusManager::new();
    focus.set_focus_wrap(FocusWrap::Wrap);
    let data = DialogData::new("Q", "m", ["A", "B"], Purpose::ConfirmDelete);
    focus.apply_focus_intent(data.show_intent());

    // index 0 -> Next -> 1 -> Next wraps -> 0
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(dialog::active_button(&focus), Some(1));
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(dialog::active_button(&focus), Some(0));
    // Prev off the first wraps to the last.
    focus.apply_focus_intent(FocusIntent::Prev);
    assert_eq!(dialog::active_button(&focus), Some(1));
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
