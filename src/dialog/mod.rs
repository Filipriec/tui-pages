//! Built-in modal dialog system (feature = `dialog`).
//!
//! The crate's focus manager already tracks an open dialog overlay, the active
//! button index, and button navigation. This module adds the missing pieces of
//! a turnkey dialog: a [`DialogData`] content type, a [`DialogResult`], and a
//! ratatui [`render_dialog`] renderer.
//!
//! Usage with [`TuiPages`](crate::TuiPages) (with the dialog content as the
//! runtime's modal payload `M`, e.g.
//! `TuiPages<View, Action, State, Pages, Handler, (), DialogData<MyPurpose>>`):
//!
//! - **Show** — return `TuiEffect::Focus(data.show_intent())` from your handler.
//! - **Navigate** — `FocusIntent::Next` / `Prev` move between buttons (handled
//!   by the focus manager automatically).
//! - **Confirm** — on your activate key, read [`selection`] to get the chosen
//!   button + purpose, act on it, then return
//!   `TuiEffect::Focus(FocusIntent::ClearOverlay)` to close.
//! - **Render** — `render_dialog(frame, area, data, active_button, &theme)`,
//!   pulling `data`/`active_button` from [`current_dialog`] / [`active_button`].

mod state;
mod ui;

pub use state::{DialogData, DialogResult};
pub use ui::{render_dialog, DialogTheme};

use crate::focus::{FocusController, FocusIntent, FocusManager, OverlayFocus};
use crossterm::event::{KeyCode, KeyEvent};

impl<D> DialogData<D> {
    /// The focus intent that opens this dialog as a modal overlay. Wrap it in
    /// [`TuiEffect::Focus`](crate::TuiEffect::Focus) (or apply it directly to a
    /// [`FocusManager`]). `O` is the app's overlay type and is inferred from
    /// the surrounding runtime.
    pub fn show_intent<O>(self) -> FocusIntent<O, DialogData<D>> {
        let buttons = self.buttons.len();
        FocusIntent::ShowModal {
            data: self,
            count: buttons,
        }
    }
}

/// The dialog currently shown by the focus manager, if any.
pub fn current_dialog<O, D>(focus: &FocusManager<O, DialogData<D>>) -> Option<&DialogData<D>> {
    match focus.overlay() {
        Some(OverlayFocus::Modal { data, .. }) => Some(data),
        _ => None,
    }
}

/// The active (highlighted) button index of the shown dialog, if any.
pub fn active_button<O, D>(focus: &FocusManager<O, DialogData<D>>) -> Option<usize> {
    match focus.overlay() {
        Some(OverlayFocus::Modal { index, .. }) => Some(*index),
        _ => None,
    }
}

/// Resolve the current dialog into a [`DialogResult`] describing the selected
/// button and the dialog's purpose. Returns `None` when no dialog is open.
pub fn selection<O, D: Clone>(focus: &FocusManager<O, DialogData<D>>) -> Option<DialogResult<D>> {
    match focus.overlay() {
        Some(OverlayFocus::Modal { data, index, .. }) => Some(DialogResult::Selected {
            purpose: data.purpose.clone(),
            index: *index,
        }),
        _ => None,
    }
}

/// What [`handle_key`] did with a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogKey<D> {
    /// No dialog was open, so the key was left untouched. Forward it to the
    /// rest of your input handling (e.g. `tui.handle_key(key, state)`).
    Ignored,
    /// The key moved the active button. The dialog stays open; redraw and wait
    /// for the next key.
    Consumed,
    /// The dialog was answered and has been closed for you. Act on the result.
    Resolved(DialogResult<D>),
}

/// Drive an open modal dialog from a raw key event, using the conventional
/// bindings so you don't have to hand-roll them:
///
/// - `Tab` / `Right` move to the next button, `Shift+Tab` / `Left` to the
///   previous (clamped, no wrap — matching the focus manager).
/// - `Enter` selects the active button and closes the dialog, returning
///   [`DialogKey::Resolved`] with [`DialogResult::Selected`].
/// - `Esc` dismisses the dialog and returns [`DialogResult::Dismissed`].
///
/// When no dialog is open it returns [`DialogKey::Ignored`] and does nothing,
/// so the typical event loop is:
///
/// ```ignore
/// match dialog::handle_key(&mut tui.focus, key) {
///     DialogKey::Ignored => { tui.handle_key(key, state)?; }
///     DialogKey::Consumed => {}
///     DialogKey::Resolved(result) => apply(result, state),
/// }
/// ```
///
/// For non-conventional bindings, drive the dialog yourself with
/// [`current_dialog`], [`active_button`], [`selection`], and
/// [`FocusIntent`](crate::FocusIntent) — this helper is just the common path.
pub fn handle_key<O: Clone + PartialEq, D: Clone>(
    focus: &mut FocusManager<O, DialogData<D>>,
    key: KeyEvent,
) -> DialogKey<D> {
    if current_dialog(focus).is_none() {
        return DialogKey::Ignored;
    }

    match key.code {
        KeyCode::Tab | KeyCode::Right => {
            focus.apply_focus_intent(FocusIntent::Next);
            DialogKey::Consumed
        }
        KeyCode::BackTab | KeyCode::Left => {
            focus.apply_focus_intent(FocusIntent::Prev);
            DialogKey::Consumed
        }
        KeyCode::Enter => {
            let result = selection(focus).unwrap_or(DialogResult::Dismissed);
            focus.apply_focus_intent(FocusIntent::ClearOverlay);
            DialogKey::Resolved(result)
        }
        KeyCode::Esc => {
            focus.apply_focus_intent(FocusIntent::ClearOverlay);
            DialogKey::Resolved(DialogResult::Dismissed)
        }
        // A modal swallows everything else while it is open.
        _ => DialogKey::Consumed,
    }
}
