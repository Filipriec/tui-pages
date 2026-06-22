# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.11] - 2026-06-22

### Added
- `DialogTheme::error_styled()` and `DialogTheme::success_styled()` — semantic error/success dialog themes using the `error` and `success` role colors from `ThemeStyles`, with background and button-active colors falling back to built-in defaults
- `render_dialog_error()`, `render_dialog_success()`, `render_dialog_error_with_button_renderer()`, `render_dialog_success_with_button_renderer()` — convenience render functions for error- and success-styled dialogs, with and without custom button renderers
- `PickerCommandQuery`, `PickerCommandClause`, `PickerCommandArgument`, `PickerCommandSpec` — structured command-query types for the picker, splitting input into scoped commands with typed arguments and selection queries
- `parse_picker_command_query()` and `parse_picker_command_query_with_specs()` — public API for parsing picker command queries with optional command specs
- `PickerScope::with_suppressed_value_completion()` — suppress value autocompletion for scopes where the typed value is already a known identifier
- `PickerScope::with_value_replacement_completion()` — autocomplete replaces the freeform value text with the completion key value rather than appending as a suffix
- `PickerScope::with_command_argument()` — mark a scope as requiring a command argument for accurate scope-boundary detection

### Changed
- **Picker query parsing refactored**: picker command parsing, scope-based autocomplete, and query handling moved into a new `src/picker/query.rs` module, replacing ad-hoc string splitting with the structured `PickerCommandQuery` parser
- `ThemeRole::Selection` fallback chain now includes `ui.selection.primary` before `ui.selection` for improved Helix theme compatibility
- `CanvasTheme` for `ThemeStyles`: `input_active()` no longer patches `text_focus` on top of `cursorline` — uses only `cursorline` for cleaner input highlight
- `update_default_cursor_style()` for `Active` mode now hides the cursor instead of showing it, making cursor visibility terminal-agnostic
- Picker results list vertical margin reduced from 1 to 0 for tighter rendering
- Canvas dependency bumped to `0.8.11`
- Error/success dialog render functions re-exported from crate root and prelude (behind `tui` feature)

### Fixed
- Selection highlight (`ui.selection.primary`) and normal text highlight resolved correctly for Helix-compatible themes
- Cursor highlight on canvas input no longer double-patches with `text_focus`, fixing contrast issues
- Picker autocomplete correctly handles suppressed-value scopes with optional suffix completion after argument-space boundaries
- Picker value-replacement autocomplete preserves previous scope clauses in multi-scope queries
- Cursor behavior made terminal-agnostic, avoiding visibility inconsistencies across terminals

## [0.8.12] - 2026-06-22

### Changed

- **Canvas render functions now render without cursor by default**: `render_canvas()` and `render_canvas_with_options()` no longer draw the canvas-owned cursor cell — they delegate to the new `_without_cursor` variants; callers that need cursor rendering should use the upstream `tui-canvas` functions directly or handle cursor management themselves
- Added `render_canvas_without_cursor()` and `render_canvas_with_options_without_cursor()` as explicit public wrappers for rendering without canvas cursor cells

## [Unreleased]

### Added
- Helix-compatible theme system (feature `tui`): `ThemeLoader` for discovering and loading Helix theme TOML files with `inherits` support, `Theme` for parsed scope-to-Style lookup with dot-delimited fallback (`ui.text.focus` → `ui.text` → `ui`), `ThemeStyles` for cached typed role styles (27 fields covering background, text, cursor, statusline, selection, menu, diagnostics, etc.), `ThemeRole` enum with all common Helix UI roles and `scopes()` fallback chains, `ThemeManager` for runtime theme management with `default_search_paths()` and hot reload via `load_ref()`, and `ThemeError` with detailed error variants (`Io`, `ParseToml`, `MissingTheme`, `InheritanceCycle`, `InvalidPaletteEntry`, `InvalidStyle`, `UnknownKey`)
- `CanvasTheme` implementation for `ThemeStyles` — all canvas surfaces (forms, textareas, textinputs, suggestions, completion) automatically derive their colors from the app's loaded Helix theme, replacing manual theme wiring
- `DialogTheme::from_theme_styles()` — derive dialog colors from the typed theme role cache, falling back to built-in defaults when a role has no color set
- `DialogTheme::themed()` — semantic purpose-driven dialog colors via `DialogPurposeClass` (Success, Failure, Neutral) and `DialogPurposeStyle` trait, letting the dialog automatically pick error/success/neutral foregrounds from the theme
- `DialogButtonRenderer` type alias and `render_dialog_with_button_renderer()` — let apps supply their own button rendering function so dialogs use the application's unified button style instead of a crate-bundled look
- `PickerTheme::from_theme_styles()` — derive picker colors from the typed theme role cache
- `TuiPages::handle_event()` — unified event handler dispatching `Key` and `Paste` crossterm events on the runtime (with and without `canvas` feature), so apps can pass a single event stream without manual matching
- `TuiPages::default_cursor_behavior()` — centralized cursor behavior query for canvas-enabled apps, factoring out cursor logic from per-app event loops
- Built-in generic fuzzy picker overlay (feature `tui`) with `nucleo`-backed ranking, scope (`%token`) parsing and autocompletion, and custom preview rendering (`PickerData`, `PickerEntry`, `PickerScope`, `PickerFieldWeights`, `PickerTheme`)
- `WorkspaceState<V, S>` and `PaneSession<V, S>` — panes now carry typed user-state `S` alongside their view
- `PaneId` and `PaneSplit` (Vertical / Horizontal) for workspace layout management
- `CommandRegistry::bind_aliases()` and `TuiPagesBuilder::command()` for binding multiple command-line aliases to a single action
- Unified `[keymap.*]` TOML-based keybinding configuration via `KeybindingConfig`
- `ActionRegistry` for mapping config action names to application action types
- Binding introspection API (`BindingCatalog`, `BindingInfo`, `BindingConflict`, `BindingSource`, `BindingLayer`) for help screens and conflict diagnostics
- `CanvasRoutingPrecedence` for tuning input dispatch between global keymap and canvas editing
- `BindingAnalysis` and `BindableActionInfo` for programmatic keybinding analysis
- `InputLayerContext` (Command / Text) for smarter routing between global keymap and canvas editing layer
- `CanvasKeybindingProfileState` with generation tracking for shared canvas keybinding profiles
- `install_keybindings()` and `is_sequence_pending()` methods on canvas host traits
- `analyze_canvas_overlaps()`, `canvas_action_name()`, `canvas_bindable_actions()`, `canvas_default_binding_catalog()`, and `canvas_suggestion_default_bindings()` for canvas binding overlap analysis and introspection
- Runtime rebind (`rebind_keymap`) and reset (`reset_keybindings_to_defaults`) on `TuiPages`
- `export_keybindings_toml()` for serializing the current keybinding state to TOML
- `keybindings_config` example demonstrating TOML-based keybinding configuration
- Keybinding inheritance: copy keybindings from one action to another across modes (e.g. Command/Enter mirrors General/Select). Uses compile-time constants instead of strings, so typos are caught by the compiler and IDEs can autocomplete.
- Canvas integration tests (`tests/canvas_integration.rs`)
- `render_canvas_unmanaged_cursor()`, `render_canvas_with_options_unmanaged_cursor()`, and `render_canvas_default_unmanaged_cursor()` — canvas render functions that skip cursor management, letting callers handle the cursor themselves in exceptional cases
- `define_buttons!` macro for generating focusable button enums with `COUNT`, `from_index`, and `index` (re-exported in prelude)
- Paste support wired by default in the canvas-provided terminal-input helpers (`CrosstermInputSession`, `CrosstermInputGuard`)

### Changed
- Canvas render functions (`render_canvas`, `render_canvas_with_options`, `render_canvas_default`) no longer manage cursor position inline — the runtime now owns cursor management centrally; use `_unmanaged_cursor` variants for exceptional cases
- `ThemeStyles`, `Theme`, `ThemeError`, `ThemeLoader`, `ThemeManager`, `ThemeRole` re-exported from crate root and prelude (behind `tui` feature)
- **Feature rename**: `dialog` feature replaced by `tui` feature, which bundles the modal dialog, the fuzzy picker, and the canvas + nucleo + unicode-width dependencies
- tui-pages migrated to new typesafe API of `tui-canvas`, eliminating raw string action names from canvas dispatch
- tui-pages runtime now owns input pipelines and canvas keybinding profiles internally
- Canvas `KeyHookKind` variants now use `CanvasKeybindingProfileHandle` instead of raw preset enums
- `CanvasTextInputHost` trait expanded with `has_keybindings`, `install_keybindings`, `is_sequence_pending`, and `accept_suggestion_suffix`
- `CanvasTextInputHost::input_key` return type changed from `CanvasTextWidgetOutcome` to `CanvasKeyDispatchOutcome`
- Canvas public API surface expanded with analysis and introspection re-exports
- `NavigationPreset::from_toml` now uses internal lenient parsing path
- Canvas dependency bumped to `0.8.10`
- Crate version bumped to `0.8.10`
- Deprecated API methods no longer panic — they emit warnings instead

### Fixed
- Focus section clamping: Prev/Next at section edges now clamp inside the section instead of escaping to adjacent targets; explicit `FocusIntent::LeaveSection` is now required to exit a section boundary
- Focused pane cannot be escaped by cursor movement anymore
- Cursor position tracking now works correctly after render pipeline refactor
- Helix theme compatibility fully implemented (Catppuccin theme palette and inheritance parsing fixed)
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

[0.8.12]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.11...v0.8.12
[0.8.11]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.10...v0.8.11
[0.8.10]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.9...v0.8.10
[0.8.9]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.8...v0.8.9
[0.8.8]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.6...v0.8.8
[0.8.6]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.5...v0.8.6
[0.8.5]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.4...v0.8.5
[0.8.4]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.3...v0.8.4
[0.8.3]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.2...v0.8.3
[Unreleased]: https://gitlab.com/filipriec/tui-pages/-/compare/v0.8.12...HEAD
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
