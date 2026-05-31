# Canvas First-Class Support Checklist

Goal: `tui-pages` should support the full `canvas` crate out of the box, with host apps only needing to enable the right feature and implement their app action conversion.

## Feature Plumbing

- [ ] Add `tui-pages` feature flags mirroring canvas capabilities: `canvas-gui`, `canvas-suggestions`, `canvas-validation`, `canvas-cursor-style`, `canvas-computed`, `canvas-textarea`, `canvas-textinput`, `canvas-keymap`, and an `canvas-all` convenience feature.
- [ ] Ensure each `tui-pages` feature enables the matching `canvas` feature and any required local dependency such as `ratatui`.
- [ ] Keep base `canvas` support optional and non-breaking for users who do not enable it.

## Public API

- [ ] Re-export all relevant canvas public types behind matching features: renderers, themes/options, suggestions, validation, computed, textarea, text input, cursor, keymap, action/result types.
- [ ] Keep exports grouped under `tui_pages::canvas` and add common reexports to `prelude` only when they are genuinely ergonomic.
- [ ] Provide stable helper names so apps do not need to import from both crates for normal usage.

## Runtime Integration

- [ ] Keep `.canvas_defaults()` as the one-call setup for standard form editor behavior.
- [ ] Add opt-in setup methods for each surface: forms, textarea, textinput, suggestions, validation, cursor style, and canvas-owned keymap handling.
- [ ] Support plain text, paste, submit, escape, tab, backspace/delete, movement, and mode changes through typed canvas actions/outcomes.
- [ ] Preserve non-canvas text routing for command bars, palettes, dialogs, and custom inputs.

## Focus And Handoff

- [ ] Support boundary handoff for `FormEditor`, `TextAreaState`, and `TextInputState`.
- [ ] Add focus helpers for top-level canvas widgets and internal canvas-only widgets.
- [ ] Define how textarea/textinput focus exits map to `FocusIntent::ExitCanvasForward` and `FocusIntent::ExitCanvasBackward`.
- [ ] Add tests for canvas fields mixed with buttons, sections, overlays, dialogs, buffers, and panes.

## Rendering

- [ ] Re-export canvas render functions and widgets behind GUI features.
- [ ] Provide `tui-pages` helper render wrappers only where they remove repeated host glue.
- [ ] Support suggestions dropdown rendering and anchor/placement helpers.
- [ ] Document cursor positioning expectations for ratatui frames.

## Suggestions

- [ ] Re-export `SuggestionItem`, `SuggestionQuery`, `SuggestionTrigger`, and suggestion dropdown renderer.
- [ ] Add default keybindings for trigger, up/down, select, and exit suggestions.
- [ ] Define how suggestion state affects input routing and focus handoff.
- [ ] Add tests covering typed filtering, selection, escape, and field transitions.

## Validation

- [ ] Re-export validation config, rules, results, masks, filters, and summaries.
- [ ] Ensure field switching and edit-mode exit can surface validation-blocked outcomes cleanly.
- [ ] Add tests for blocked navigation, valid navigation, display masks, and character filters.

## Cursor Style

- [ ] Re-export cursor style helpers.
- [ ] Provide opt-in runtime hooks for mode-based cursor style updates.
- [ ] Ensure cursor updates never run unless the feature is enabled and the host opts in.

## Textarea

- [ ] Re-export textarea state, provider, editor, widget, overflow options, and event outcomes.
- [ ] Add textarea-specific key/paste mapping, including newline insertion.
- [ ] Support textarea render helpers, scrolling, wrapping, and optional syntax highlight features.
- [ ] Add tests for multiline editing, paste, movement, wrapping/overflow state, and focus exit.

## Text Input

- [ ] Re-export text input state, provider, editor, widget, and event outcomes.
- [ ] Add textinput-specific key/paste mapping, including submit handling.
- [ ] Support suggestion suffix handling.
- [ ] Add tests for typing, paste, backspace/delete, submit, tab suggestion accept, and focus exit.

## Canvas Keymap

- [ ] Support delegating raw `KeyEvent` handling to `canvas::CanvasKeyMap` when `canvas-keymap` is enabled.
- [ ] Map `KeyEventOutcome` into `TuiPagesStatus`, `CanvasAction`, or `FocusIntent` without app-side glue.
- [ ] Keep existing `tui-pages` keymap flow available for apps that prefer centralized bindings.

## Examples And Docs

- [ ] Add minimal examples for form editor, suggestions, validation, cursor style, textarea, textinput, and keymap integration.
- [ ] Add one full example using multiple canvas widgets with normal `tui-pages` navigation.
- [ ] Document the recommended app action shape: `enum Action { Canvas(CanvasAction), ... }` plus `From<CanvasAction>`.
- [ ] Document feature selection and the smallest feature set for each widget type.

## Compatibility

- [ ] Migrate client-side canvas glue into `tui-pages` where it is generic.
- [ ] Keep app-specific logic, async fetching, persistence, and business side effects outside `tui-pages`.
- [ ] Add compile tests or integration tests for common feature combinations: base, gui, suggestions, validation, textarea, textinput, keymap, all.

## Done Definition

- [ ] A new app can use every public canvas surface through `tui-pages` without importing canvas directly for standard workflows.
- [ ] All canvas feature combinations compile.
- [ ] Each supported surface has at least one integration test and one example.
- [ ] Existing `tui-pages` behavior without canvas features remains unchanged.
