//! Built-in fuzzy picker (feature = `tui`).
//!
//! A turnkey fuzzy-search overlay: a list of [`PickerEntry`] rows, a canvas
//! text input with scope (`%token`) parsing and autocompletion, a
//! [`nucleo`](https://docs.rs/nucleo)-backed ranking engine, and a ratatui
//! renderer.
//!
//! [`PickerData`] is generic over an application-owned mode payload `M`
//! (see [`PickerData::set_mode`]) that the library stores but never inspects —
//! mirroring how [`DialogData`](crate::dialog::DialogData) carries an opaque
//! purpose. Use `PickerData<()>` (the default) when you don't need one.
//!
//! ## Scopes
//!
//! A [`PickerScope`] binds a `%token` to a set of entry field keys, so
//! `%profile finance` ranks the `profile` field against `finance`. Scopes can
//! declare aliases, a completion key (for the inline suffix), a value-token
//! limit, and — via [`PickerScope::with_suppressed_value_completion`] — opt out
//! of inline completion for free-form values.
//!
//! ## Ranking
//!
//! Field keys are weighted by [`PickerFieldWeights`]; the default keeps `label`
//! heaviest. Applications layer their own domain keys on top with
//! [`PickerFieldWeights::with_override`] and
//! [`PickerData::with_field_weights`].

mod query;
mod state;
mod ui;

pub use query::{
    PickerCommandArgument, PickerCommandClause, PickerCommandQuery, PickerCommandSpec,
    parse_picker_command_query, parse_picker_command_query_with_specs,
};
pub use state::{
    PickerData, PickerEntry, PickerField, PickerFieldWeights, PickerHead, PickerHeadColumn,
    PickerHeadColumnBuilder, PickerInputProvider, PickerLayout, PickerScope,
};
pub use ui::{PickerTheme, centered_picker_area, render_picker, render_picker_with_custom_preview};
