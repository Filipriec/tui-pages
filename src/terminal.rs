//! Panic-safe terminal lifecycle.
//!
//! Every example's `main` opened the alternate screen and enabled raw mode by
//! hand, then restored them after the loop — which means a panic *inside* the
//! loop left the user's terminal wrecked (no echo, stuck in the alternate
//! screen). [`enter`] returns a guard that restores the terminal on drop, so
//! teardown happens on the normal path and while unwinding alike. The crate
//! owns the ceremony; the application owns the draw loop.
//!
//! ```ignore
//! let _guard = tui_pages::terminal::enter()?;
//! let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
//! loop { /* draw + handle_key */ }
//! // `_guard` restores the terminal here — or if the loop panics.
//! ```
//!
//! The guard targets `stderr`, leaving `stdout` free for piping the app's
//! output. This needs only `crossterm`, which the crate already depends on; no
//! rendering backend is pulled in.

use std::io;

use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

/// Enable raw mode, switch `stderr` to the alternate screen, and turn on
/// bracketed paste, returning a guard that reverses all three when dropped.
/// Hold the guard for the lifetime of the UI; let it drop (or be dropped by
/// unwinding) to restore the terminal.
///
/// Bracketed paste makes the terminal deliver a paste as a single
/// `Event::Paste(text)` rather than a flood of synthetic key presses, so a text
/// widget can insert it in one shot. Forward that event to
/// [`crate::runtime::TuiPages::handle_paste`].
pub fn enter() -> io::Result<TerminalGuard> {
    enable_raw_mode()?;
    execute!(io::stderr(), EnterAlternateScreen, EnableBracketedPaste)?;
    Ok(TerminalGuard { _private: () })
}

/// Restores the terminal — leaves the alternate screen and disables raw mode —
/// when dropped. Created by [`enter`].
#[must_use = "dropping the guard immediately restores the terminal; bind it for the UI's lifetime"]
pub struct TerminalGuard {
    // Keeps the type unconstructable outside this module, so the only way to
    // get one is through `enter`, which paired the setup with this teardown.
    _private: (),
}

impl TerminalGuard {
    /// Restore the terminal now, surfacing any error instead of swallowing it
    /// as the `Drop` path does. After this the guard is inert.
    pub fn restore(self) -> io::Result<()> {
        // `restore_terminal` is idempotent, and `Drop` will call it again
        // harmlessly after this returns.
        restore_terminal()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best effort on the teardown path: a terminal we cannot restore is
        // nothing we can report from `drop`.
        let _ = restore_terminal();
    }
}

fn restore_terminal() -> io::Result<()> {
    execute!(io::stderr(), DisableBracketedPaste, LeaveAlternateScreen)?;
    disable_raw_mode()
}
