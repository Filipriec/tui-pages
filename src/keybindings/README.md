# Editor keybinding presets

Page-level default keymaps for **tui-pages** (focus, quit, buffers, panes).

## Layout

| File | Purpose |
|------|---------|
| `action.rs` | Shared [`NavigationAction`] enum and `try_standard_navigation_action` |
| `preset.rs` | TOML preset loader and validation |
| `presets/vim.toml` | Vim key data → `.vim_defaults()` |
| `presets/emacs.toml` | GNU Emacs key data → `.emacs_defaults()` |
| `presets/helix.toml` | Helix key data → `.helix_defaults()` |
| `vim.rs`, `emacs.rs`, `helix.rs` | Thin Rust wrappers around the bundled TOML presets |

## Add another editor

1. Add `presets/kakoune.toml` using existing [`NavigationAction`] names.
2. Add a small `kakoune.rs` wrapper if you want `.kakoune_defaults()`.
3. Add `mod kakoune;` and `pub use kakoune::…` in `mod.rs`.
4. Re-export from `src/lib.rs` if you want `tui_pages::…` at the crate root.

For user-editable keymaps, load a TOML string with `NavigationPreset::from_toml`,
`apply_navigation_preset_toml`, `remap_navigation_preset_toml`, or the builder's
`.navigation_preset_toml(...)` / `.remap_navigation_preset_toml(...)`. The
`apply` functions add bindings; the `remap` functions first remove old bindings
for the actions named in the TOML. Preset sections are mode-aware:

```toml
[general]
mode = "general"
focus_next = ["j", "down", "tab"]
focus_prev = ["k", "up", "shift+tab"]

[global]
mode = "global"
quit = ["ctrl+c"]
```

For runtime end-user settings, call `tui.remap_navigation_preset_toml(...)`
after reading the user's config. The method validates before mutating the live
keymap and resets any in-progress multi-key sequence after a successful remap.

Pick **one** preset per app (do not combine `.vim_defaults()` and `.emacs_defaults()` on the same mode).

Canvas field editing lives in `src/canvas.rs`, not here.
