//! Built-in modal dialog system (feature = `dialog`).
//!
//! The crate's focus manager already tracks an open dialog overlay, the active
//! button index, and button navigation. This module adds the missing pieces of
//! a turnkey dialog: a [`DialogData`] content type, a [`DialogResult`], and a
//! ratatui [`render_dialog`] renderer.
//!
//! Usage with [`TuiPages`](crate::TuiPages) (with the dialog payload as the
//! runtime's `D` type, e.g. `TuiPages<View, Action, State, _, _, DialogData<MyPurpose>>`):
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

use crate::focus::{FocusIntent, FocusManager, OverlayFocus};

impl<D> DialogData<D> {
    /// The focus intent that opens this dialog as a modal overlay. Wrap it in
    /// [`TuiEffect::Focus`](crate::TuiEffect::Focus) (or apply it directly to a
    /// [`FocusManager`]).
    pub fn show_intent<P>(self) -> FocusIntent<DialogData<D>, P> {
        let buttons = self.buttons.len();
        FocusIntent::ShowDialog {
            data: self,
            buttons,
        }
    }
}

/// The dialog currently shown by the focus manager, if any.
pub fn current_dialog<D, P>(focus: &FocusManager<DialogData<D>, P>) -> Option<&DialogData<D>> {
    match focus.overlay() {
        Some(OverlayFocus::Dialog { data, .. }) => Some(data),
        _ => None,
    }
}

/// The active (highlighted) button index of the shown dialog, if any.
pub fn active_button<D, P>(focus: &FocusManager<DialogData<D>, P>) -> Option<usize> {
    match focus.overlay() {
        Some(OverlayFocus::Dialog { index, .. }) => Some(*index),
        _ => None,
    }
}

/// Resolve the current dialog into a [`DialogResult`] describing the selected
/// button and the dialog's purpose. Returns `None` when no dialog is open.
pub fn selection<D: Clone, P>(focus: &FocusManager<DialogData<D>, P>) -> Option<DialogResult<D>> {
    match focus.overlay() {
        Some(OverlayFocus::Dialog { data, index, .. }) => Some(DialogResult::Selected {
            purpose: data.purpose.clone(),
            index: *index,
        }),
        _ => None,
    }
}
