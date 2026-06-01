# Editor keybinding presets

Page-level default keymaps for **tui-pages** (focus, quit, buffers, panes).

## Layout

| File | Purpose |
|------|---------|
| `action.rs` | Shared [`NavigationAction`] enum and `try_standard_navigation_action` |
| `vim.rs` | Vim keys → `.vim_defaults()` |
| `emacs.rs` | GNU Emacs keys → `.emacs_defaults()` |
| `helix.rs` | Helix keys → `.helix_defaults()` |

## Add another editor

1. Copy `vim.rs` to `kakoune.rs` (or similar).
2. Implement `bind_*_defaults` using `super::action::{bind_str, NavigationAction}`.
3. Add `mod kakoune;` and `pub use kakoune::…` in `mod.rs`.
4. Re-export from `src/lib.rs` if you want `tui_pages::…` at the crate root.

Pick **one** preset per app (do not combine `.vim_defaults()` and `.emacs_defaults()` on the same mode).

Canvas field editing lives in `src/canvas.rs`, not here.
