# Canvas First-Class Support Checklist

Goal: `tui-pages` should support the full `canvas` crate out of the box, with host apps only needing to enable the right feature and implement their app action conversion.

## Feature Plumbing

> Design decision: canvas support is **not** split into sub-features. A single
> `canvas` feature turns on every canvas surface at once (full support). There
> is no `canvas-gui`/`canvas-validation`/etc. — enabling `canvas` enables them
> all.

- [x] Add a single `canvas` feature that enables every canvas capability (GUI, suggestions, cursor style, validation, computed, textarea, textinput, keymap) in one switch.
- [x] Ensure `canvas` enables every matching `canvas/*` feature plus `ratatui`. (Canvas's default `textmode-vim` rides along, satisfying its "exactly one text mode" contract.)
- [x] Keep `canvas` support optional and non-breaking for users who do not enable it. (Default build has no canvas; verified.)

## Public API

- [x] Re-export all relevant canvas public types behind matching features: renderers, themes/options, suggestions, validation, computed, textarea, text input, cursor, keymap, action/result types.
- [x] Keep exports grouped under `tui_pages::canvas` and add common reexports to `prelude` only when they are genuinely ergonomic. (Feature-gated types live under `tui_pages::canvas`; only `CanvasAction`/`CanvasDispatchOutcome`/`dispatch_canvas_action` are in the prelude.)
- [x] Provide stable helper names so apps do not need to import from both crates for normal usage. (Includes `*_for_host` handoff helpers and `CrosstermInput*` session helpers behind the relevant features.)

## Runtime Integration

- [x] Keep `.canvas_defaults()` as the one-call setup for standard form editor behavior. It installs canvas keymaps, typed text routing, and suggestion keys.
- [x] Add opt-in setup methods for each surface: forms, textarea, textinput, suggestions, validation, cursor style, and canvas-owned keymap handling.
- [x] Support plain text, paste, submit, escape, tab, backspace/delete, movement, and mode changes through typed canvas actions/outcomes. Form editors use `CanvasAction`; textinput/textarea use typed widget outcomes.
- [x] Preserve non-canvas text routing for command bars, palettes, dialogs, and custom inputs. The text mapper only runs for canvas focus targets and canvas-accepting page specs.

## Focus And Handoff

- [x] Support boundary handoff for `FormEditor`, `TextAreaState`, and `TextInputState`.
- [x] Add focus helpers for top-level canvas widgets and internal canvas-only widgets.
- [x] Define how textarea/textinput focus exits map to `FocusIntent::ExitCanvasForward` and `FocusIntent::ExitCanvasBackward`.
- [x] Add tests for canvas fields mixed with buttons, sections, overlays, dialogs, buffers, and panes.

## Rendering

- [x] Re-export canvas render functions and widgets.
- [x] Provide `tui-pages` helper render wrappers only where they remove repeated host glue.
- [x] Support suggestions dropdown rendering and anchor/placement helpers.
- [x] Document cursor positioning expectations for ratatui frames.

## Suggestions

- [x] Re-export `SuggestionItem`, `SuggestionQuery`, `SuggestionTrigger`, and suggestion dropdown renderer.
- [x] Add default keybindings for trigger, up/down, select, and exit suggestions.
- [x] Define how suggestion state affects input routing and focus handoff.
- [x] Add tests covering typed filtering, selection, escape, and field transitions.

## Validation

- [x] Re-export validation config, rules, results, masks, filters, and summaries.
- [x] Ensure field switching and edit-mode exit can surface validation-blocked outcomes cleanly.
- [x] Add tests for blocked navigation, valid navigation, display masks, and character filters.

## Cursor Style

- [x] Re-export cursor style helpers.
- [x] Provide opt-in runtime hooks for mode-based cursor style updates.
- [x] Ensure cursor updates never run unless the feature is enabled and the host opts in.

## Textarea

- [x] Re-export textarea state, provider, editor, widget, overflow options, and event outcomes.
- [x] Add textarea-specific key/paste mapping, including newline insertion.
- [x] Support textarea render helpers, scrolling, wrapping, and optional syntax highlight features.
- [x] Add tests for multiline editing, paste, movement, wrapping/overflow state, and focus exit.

## Text Input

- [x] Re-export text input state, provider, editor, widget, and event outcomes.
- [x] Add textinput-specific key/paste mapping, including submit handling.
- [x] Support suggestion suffix handling.
- [x] Add tests for typing, paste, backspace/delete, submit, tab suggestion accept, and focus exit.

## Canvas Keymap

- [x] Support delegating raw `KeyEvent` handling to `canvas::CanvasKeyMap` when `canvas` is enabled.
- [x] Map `KeyEventOutcome` into `TuiPagesStatus`, `CanvasAction`, or `FocusIntent` without app-side glue.
- [x] Keep existing `tui-pages` keymap flow available for apps that prefer centralized bindings.

## Examples And Docs

- [x] Add examples for form editor, suggestions, validation, cursor style, textarea, textinput, and keymap integration under `examples/canvas`: `minimal`, `textarea`, and `full`.
- [x] Add one full example using multiple canvas widgets with normal `tui-pages` navigation and one canvas-focused page per surface.
- [x] Document the recommended app action shape: `enum Action { Canvas(CanvasAction), ... }` plus `From<CanvasAction>`.
- [x] Document feature selection and the smallest feature set for each widget type.

## Compatibility

- [x] Migrate client-side canvas glue into `tui-pages` where it is generic. Generic action/key/widget handoff and render anchoring has moved; app-specific side effects stay outside.
- [x] Keep app-specific logic, async fetching, persistence, and business side effects outside `tui-pages`.
- [x] Add compile/integration tests for canvas. Since support is a single `canvas` feature (no split), `tests/canvas_integration.rs` covers the whole surface: typed-text routing, default keymaps, focus-boundary handoff, `PageSpec` mode sync, editor `execute`, suggestions, validation, computed fields, textinput typing/paste/suggestion-suffix, textarea/textinput focus exits, canvas keymap dispatch, plus a compile-time proof that every canvas surface (GUI, suggestions, validation, computed, textarea, textinput, keymap, cursor, crossterm session) is reachable through `tui_pages::canvas`. 25 tests, all green; the file is `#![cfg(feature = "canvas")]` so the default build is unaffected.

## Done Definition

- [x] A new app can use every public canvas surface through `tui-pages` without importing canvas directly for standard workflows.
- [x] All canvas feature combinations compile. Canvas support is one unified feature; `cargo test --all-features` covers it with the rest of the crate.
- [x] Each supported surface has at least one integration test and one example.
- [x] Existing `tui-pages` behavior without canvas features remains unchanged.
