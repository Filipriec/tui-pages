# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Unified `[keymap.*]` TOML-based keybinding configuration via `KeybindingConfig`
- `ActionRegistry` for mapping config action names to application action types
- Binding introspection API (`BindingCatalog`, `BindingInfo`, `BindingConflict`, `BindingSource`, `CanvasRoutingPrecedence`) for help screens and conflict diagnostics
- `InputLayerContext` (Command / Text) for smarter routing between global keymap and canvas editing layer
- `CanvasKeybindingProfileState` with generation tracking for shared canvas keybinding profiles
- `install_keybindings()` and `is_sequence_pending()` methods on canvas host traits
- `analyze_canvas_overlaps()` and related public API for canvas binding overlap analysis
- Runtime rebind (`rebind_keymap`) and reset (`reset_keybindings_to_defaults`) on `TuiPages`
- `export_keybindings_toml()` for serializing the current keybinding state to TOML
- `keybindings_config` example demonstrating TOML-based keybinding configuration
- Canvas integration tests (`tests/canvas_integration.rs`)

### Changed
- tui-pages runtime now owns input pipelines and canvas keybinding profiles internally
- Canvas `KeyHookKind` variants now use `CanvasKeybindingProfileHandle` instead of raw preset enums
- `CanvasTextInputHost` trait expanded with `has_keybindings`, `install_keybindings`, `is_sequence_pending`, and `accept_suggestion_suffix`
- `CanvasTextInputHost::input_key` return type changed from `CanvasTextWidgetOutcome` to `CanvasKeyDispatchOutcome`
- Canvas public API surface expanded with analysis and introspection re-exports
- `NavigationPreset::from_toml` now uses internal lenient parsing path

### Fixed
- Pipeline issues with input sequence tracker resets
- Missed error conditions in input handling

## [0.8.2]

### Changed
- Rust edition bumped to 2024

## [0.8.1]

### Added
- Canvas high-level API for forms, textareas, and bottom panels
- mdbook canvas documentation support
- Examples for high-level canvas API (`form_helix`, `form_vim`, `textarea_helix_minimal`, `textarea_vim_minimal`)
- Paste support in canvas textareas via clipboard integration
- Exit-pane now automatically shifts focus to the next available pane
- Dialog keybindings are now preset-driven (Vim, Emacs, Helix)
- Re-exported canvas keybinding profile APIs for downstream consumers

### Changed
- Canvas crate internals reworked to v0.8.0 with cleaner lifetime management
- General preset bindings reused across dialogs instead of per-preset duplicates

### Fixed
- Textarea high-level API line count not properly tracked on text operations

## [0.7.5] - 2026-06-05

### Added
- Customizable form row widths per row
- Customizable field widths per form field
- Bottom panel component built from the canvas crate
- Undo and redo support in canvas text areas
- Keybinding production version with default presets

### Changed
- Removed unnecessary lifetimes in core types
- Cleaned up keybinding integration with the canvas crate
- Examples now use default keybinding presets

## [0.7.4] - 2026-06-01

### Added
- First-class canvas crate support integrated into tui-pages
- Full canvas example with form, textarea, and suggestion surfaces
- Vim, Emacs, and Helix keybinding presets for page navigation
- Top-level `keybindings/` directory with TOML-based presets and README
- Textarea suggestion support via canvas input

### Changed
- Textarea migrated to new canvas API
- Removed deprecated box-based API in favor of canvas traits

## [0.7.3] - 2026-05-31

### Added
- Professional-grade README

### Fixed
- Production input issues
- KeyChord and sequence handling requiring `+` separator

## [0.7.2] - 2026-05-30

### Added
- mdbook documentation improvements
- Professional README with adjusted release tags

## [0.7.1] - 2026-05-30

### Fixed
- Examples and mdbook fixed for production use
- README typo corrected

## [0.7.0] - 2026-05-30

### Added
- New architecture with modular page system
- Minimal and full working examples
- Built-in dialog support rendering out of the box
- Buffer-based example and generalization
- Page function alias (`PageFn`) for improved UX
- Clamping and wrapping modes now configurable by the user
- KeyChord support with composable sequences
- mdbook documentation with Catppuccin theme

### Changed
- Dialog and picker components are no longer hardcoded
- Modes system adjusted for better ergonomics
- README rewritten with professional style
- License added (MIT)

## [0.1.1] - 2026-05-26

### Added
- Error handling improvements
- More generalization across components
- Demo example

### Fixed
- Shift+Tab not working in tab navigation

## [0.1.0] - 2026-05-25

### Added
- Initial port from the client library
- Core TUI multi-page navigation system

[Unreleased]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.2...HEAD
[0.8.2]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.1...v0.8.2
[0.8.1]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.7.5...v0.8.1
[0.7.5]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.7.4...v0.7.5
[0.7.4]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.7.3...v0.7.4
[0.7.3]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.7.2...v0.7.3
[0.7.2]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.7.1...v0.7.2
[0.7.1]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.7.0...v0.7.1
[0.7.0]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.1.1...v0.7.0
[0.1.1]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.1.0...v0.1.1
[0.1.0]: https://gitlab.com/filipriec/tui-pages/-/tree/v0.1.0
