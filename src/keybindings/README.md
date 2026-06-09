# Editor keybinding presets

WARNING - canvas crate keybindings are being used, not passed into this crate

Page-level default keymaps for **tui-pages** (focus, quit, buffers, panes).

## Layout

| File | Purpose |
|------|---------|
| `action.rs` | Shared [`NavigationAction`] enum and `try_standard_navigation_action` |
| `preset.rs` | TOML preset loader and validation |
| `builtin.rs` | Shared Rust wrappers for all bundled editor presets |
| `presets/vim.toml` | Vim key data → `.vim_defaults()` |
| `presets/emacs.toml` | GNU Emacs key data → `.emacs_defaults()` |
| `presets/helix.toml` | Helix key data → `.helix_defaults()` |

## Add another editor

1. Add `presets/kakoune.toml` using existing [`NavigationAction`] names.
2. Add a `BuiltinNavigationPreset` variant and wire its TOML in `builtin.rs`.
3. Add small compatibility helpers only if you want `.kakoune_defaults()`.
4. Re-export from `src/lib.rs` if you want `tui_pages::…` at the crate root.

For user-editable keymaps, load a TOML string with `NavigationPreset::from_toml`,
`apply_navigation_preset_toml`, `remap_navigation_preset_toml`, or the builder's
`.navigation_preset_toml(...)` / `.remap_navigation_preset_toml(...)`. The
`apply` functions add bindings; the `remap` functions first remove old bindings
for the actions named in the TOML. The public `vim_preset_toml()`,
`emacs_preset_toml()`, and `helix_preset_toml()` helpers expose starter files
an app can write into a user config directory. Preset sections are mode-aware:

```toml
[general]
mode = "general"
focus_next = ["j", "down", "tab"]
focus_prev = ["k", "up", "shift+tab"]

[global]
mode = "global"
quit = ["ctrl+c"]

[dialog]
mode = "dialog"
focus_next = ["tab", "right"]
focus_prev = ["shift+tab", "backtab", "left"]
activate = ["enter"]
leave_section = ["esc"]
```

For runtime end-user settings, call `tui.remap_navigation_preset_toml(...)`
after reading the user's config. The method validates before mutating the live
keymap and resets any in-progress multi-key sequence after a successful remap.
If you use the built-in dialog helper, keep a `dialog::DialogKeyBindings` in
your app state and rebuild/remap it from the same TOML so the `[dialog]` section
drives modal navigation and dismissal too.

Validation returns `NavigationPresetError::Issues(Vec<NavigationPresetIssue>)`
when the file has one or more problems. The user-facing TOML helpers also emit a
`tracing::warn!`. Issues include unknown actions, bad key strings, empty binding
lists, duplicate bindings inside the TOML, and conflicts with live bindings that
would remain after a remap.

Settings UIs can discover supported actions with `navigation_action_infos()` or
`NavigationAction::infos()`. Each item includes the TOML name, display label,
description, and category.

Pick **one** preset per app (do not combine `.vim_defaults()` and `.emacs_defaults()` on the same mode).

Canvas field editing lives in `src/canvas.rs`, not here.
