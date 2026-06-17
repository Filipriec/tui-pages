# Helix-Compatible Theme Loader Plan

This plan is for implementing first-class theming in `tui-pages` by reading Helix theme TOML files and exposing parsed `ratatui::style::Style` values.

Do not copy Helix source code. Reimplement the behavior from the public theme file format so this crate can stay MIT licensed.

Do not run `cargo fmt` unless the user explicitly asks.

## Goal

Implement a small theme module that can:

1. Read a Helix theme TOML file from disk.
2. Parse Helix-style color palette entries.
3. Parse Helix-style scope entries into `ratatui::style::Style`.
4. Support `inherits = "parent_theme"` by recursively loading parent theme files.
5. Expose simple style lookup by scope name, with Helix-style dot fallback.

Example target API:

```rust
use tui_pages::theme::{Theme, ThemeLoader};

let loader = ThemeLoader::new(["/home/me/.config/helix/themes"]);
let theme = loader.load("catppuccin_mocha")?;

let text = theme.get("ui.text");
let focused = theme.get("ui.text.focus");
let border = theme.get("ui.border");
```

## Dependencies

Use the dependencies already present in `Cargo.toml`:

- `toml` for parsing theme files.
- `ratatui` for `Color`, `Modifier`, and `Style`.

Use standard library APIs for file IO:

- `std::fs::read_to_string`
- `std::path::{Path, PathBuf}`
- `std::collections::{HashMap, HashSet}`

Do not add `serde` for the theme parser. Use `toml::Value` directly because Helix themes use arbitrary scope names as TOML keys, such as `"ui.text"` and `"keyword.control.repeat"`.

If a custom error type is needed, implement it manually with `std::fmt::Display` and `std::error::Error`. Do not add a new error crate unless there is a strong reason.

## Feature Gate

The theme API must only compile when `ratatui` is available.

Recommended approach:

1. Add `src/theme.rs`.
2. In `src/lib.rs`, export it behind the existing `tui` feature first:

```rust
#[cfg(feature = "tui")]
pub mod theme;

#[cfg(feature = "tui")]
pub use theme::{Theme, ThemeError, ThemeLoader};
```

This is enough for now because the output type is `ratatui::style::Style`.

Later, if needed, create a narrower `theme` feature:

```toml
theme = ["dep:ratatui"]
tui = ["theme", ...]
```

Do not do that extra feature split in the first implementation unless it is clearly needed.

## File Layout

Create:

```text
src/theme.rs
```

Optional later, only if the file gets too large:

```text
src/theme/color.rs
src/theme/loader.rs
src/theme/parser.rs
```

For the first pass, keep everything in `src/theme.rs`.

## Public Types

Implement these public types:

```rust
#[derive(Debug, Clone)]
pub struct ThemeLoader {
    theme_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Theme {
    name: String,
    styles: HashMap<String, Style>,
}

#[derive(Debug)]
pub enum ThemeError {
    Io { path: PathBuf, source: std::io::Error },
    ParseToml { path: PathBuf, source: toml::de::Error },
    MissingTheme { name: String },
    InheritanceCycle { name: String },
    InvalidThemeRoot,
    InvalidInherits { value: toml::Value },
    InvalidPaletteEntry { name: String, value: toml::Value },
    InvalidStyle { scope: String, reason: String },
}
```

Import these ratatui types:

```rust
use ratatui::style::{Color, Modifier, Style};
```

## Public API

Implement:

```rust
impl ThemeLoader {
    pub fn new<I, P>(theme_dirs: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>;

    pub fn load(&self, name: &str) -> Result<Theme, ThemeError>;

    pub fn load_path(&self, path: impl AsRef<Path>) -> Result<Theme, ThemeError>;

    pub fn read_names(&self) -> Vec<String>;
}

impl Theme {
    pub fn name(&self) -> &str;

    pub fn get(&self, scope: &str) -> Style;

    pub fn try_get(&self, scope: &str) -> Option<Style>;

    pub fn try_get_exact(&self, scope: &str) -> Option<Style>;

    pub fn styles(&self) -> &HashMap<String, Style>;
}
```

`Theme::get` must return `Style::default()` if no style exists.

`Theme::try_get` must use Helix-style dot fallback:

```text
ui.text.focus -> ui.text -> ui
```

## Theme File Discovery

`ThemeLoader::new` accepts directories that contain `.toml` theme files directly.

Example:

```rust
let loader = ThemeLoader::new([
    "/home/me/.config/helix/themes",
    "/usr/share/helix/runtime/themes",
]);
```

`load("catppuccin_mocha")` searches:

```text
<dir>/catppuccin_mocha.toml
```

Search directories in the order provided. First match wins.

Do not automatically append `themes` inside `ThemeLoader::new`. The caller should pass the exact directories. This is simpler and avoids guessing the app config layout.

`read_names` should scan all configured directories, collect `.toml` file stems, deduplicate them, and return sorted names.

## TOML Reading

Use:

```rust
let raw = std::fs::read_to_string(&path)?;
let value = toml::from_str::<toml::Value>(&raw)?;
```

The root value must be a TOML table.

Reject any non-table root with `ThemeError::InvalidThemeRoot`.

## Supported Helix Syntax

Support this shape:

```toml
inherits = "parent_theme"

"ui.background" = { bg = "base" }
"ui.text" = { fg = "text" }
"ui.text.focus" = { fg = "accent", modifiers = ["bold"] }
"keyword" = "purple"

[palette]
base = "#1e1e2e"
text = "#cdd6f4"
accent = "#89b4fa"
purple = "#cba6f7"
```

Style values can be either:

```toml
"scope" = "color-name"
```

or:

```toml
"scope" = { fg = "color-name", bg = "color-name", modifiers = ["bold"], underline = { color = "red" } }
```

## Palette Parsing

Build a palette map before parsing styles.

Start with these built-in names:

```text
default     -> Color::Reset
black       -> Color::Black
red         -> Color::Red
green       -> Color::Green
yellow      -> Color::Yellow
blue        -> Color::Blue
magenta     -> Color::Magenta
cyan        -> Color::Cyan
gray        -> Color::Gray
light-red   -> Color::LightRed
light-green -> Color::LightGreen
light-yellow -> Color::LightYellow
light-blue  -> Color::LightBlue
light-magenta -> Color::LightMagenta
light-cyan  -> Color::LightCyan
light-gray  -> Color::DarkGray
white       -> Color::White
```

Note: Helix calls bright black `light-gray`. Ratatui calls it `DarkGray`, so map `light-gray` to `Color::DarkGray`.

Then merge `[palette]` entries from the theme. Palette values must be strings.

Supported color strings:

1. Hex RGB: `"#rrggbb"`
2. ANSI 256-color index: `"0"` through `"255"`
3. Existing palette color name when parsing style values

Implement hex parsing manually:

```text
#ffffff -> Color::Rgb(255, 255, 255)
```

Reject malformed hex with a clear `ThemeError::InvalidPaletteEntry` or `ThemeError::InvalidStyle`.

## Style Parsing

Implement:

```rust
fn parse_style(scope: &str, value: toml::Value, palette: &HashMap<String, Color>) -> Result<Style, ThemeError>
```

Rules:

1. If `value` is a string, parse it as a foreground color:

```rust
Style::default().fg(color)
```

2. If `value` is a table, support these keys:

```text
fg
bg
modifiers
underline
```

3. `fg` and `bg` must be strings and map through `parse_color`.

4. `modifiers` must be an array of strings.

5. `underline` must be a table. Support `color` now. Ignore `style` for now unless ratatui exposes underline style in the current version.

6. Unknown style table keys should be an error. This catches typos early.

## Modifier Mapping

Map Helix modifier strings to ratatui modifiers:

```text
bold        -> Modifier::BOLD
dim         -> Modifier::DIM
italic      -> Modifier::ITALIC
underlined  -> Modifier::UNDERLINED
slow_blink  -> Modifier::SLOW_BLINK
rapid_blink -> Modifier::RAPID_BLINK
reversed    -> Modifier::REVERSED
hidden      -> Modifier::HIDDEN
crossed_out -> Modifier::CROSSED_OUT
```

Also accept kebab-case aliases for compatibility:

```text
slow-blink  -> Modifier::SLOW_BLINK
rapid-blink -> Modifier::RAPID_BLINK
crossed-out -> Modifier::CROSSED_OUT
```

Apply modifiers with:

```rust
style = style.add_modifier(modifier);
```

## Underline Mapping

Ratatui `Style` supports underline color:

```rust
style = style.underline_color(color);
```

Helix supports:

```toml
underline = { color = "red", style = "curl" }
```

For the first implementation:

1. Apply `underline.color` with `Style::underline_color`.
2. If an underline table is present, also add `Modifier::UNDERLINED`.
3. Parse and ignore `underline.style` values because ratatui currently does not expose Helix's underline style variants in the same way.
4. Error on unknown underline keys.

## Inheritance

Support:

```toml
inherits = "parent_theme"
```

Algorithm:

1. `load(name)` finds `name.toml`.
2. Parse that file into `toml::Value`.
3. If it has `inherits`, recursively load the parent TOML value by theme name.
4. Merge parent and child.
5. Parse the merged TOML into `Theme`.

Cycle detection:

```rust
fn load_value(name: &str, visited: &mut HashSet<String>) -> Result<toml::Value, ThemeError>
```

If `visited` already contains `name`, return `ThemeError::InheritanceCycle`.

Merge rules:

1. Child keys override parent keys.
2. `[palette]` is merged separately so child palette entries override parent palette entries without deleting unrelated parent colors.
3. Remove `inherits` before final style parsing.

Example:

```toml
# parent
"ui.text" = { fg = "text" }
[palette]
text = "#ffffff"
base = "#000000"
```

```toml
# child
inherits = "parent"
[palette]
text = "#eeeeee"
```

Final palette:

```text
text = #eeeeee
base = #000000
```

Final styles:

```text
ui.text inherited from parent
```

## Merge Implementation

Implement a small recursive table merge:

```rust
fn merge_theme_values(parent: toml::Value, child: toml::Value) -> toml::Value
```

Required behavior:

1. Both values should be tables by this point.
2. Clone or move the parent table.
3. For each child key except `palette`, overwrite the parent key.
4. For `palette`, merge parent palette table and child palette table entry by entry.
5. Child palette entries override parent palette entries.

Do not over-engineer generic TOML merging. This loader only needs theme merging.

## Parsing Final TOML Into Theme

After inheritance merge:

1. Root must be a table.
2. Remove `inherits`.
3. Remove `palette` and parse it.
4. For every remaining root key, parse it as a style scope.
5. Store parsed styles in `HashMap<String, Style>`.

Skip no style silently. If a style entry is malformed, return an error.

## Scope Lookup

Implement exact lookup:

```rust
pub fn try_get_exact(&self, scope: &str) -> Option<Style> {
    self.styles.get(scope).copied()
}
```

Implement fallback lookup:

```rust
pub fn try_get(&self, scope: &str) -> Option<Style> {
    let mut current = scope;
    loop {
        if let Some(style) = self.try_get_exact(current) {
            return Some(style);
        }
        let Some((parent, _)) = current.rsplit_once('.') else {
            return None;
        };
        current = parent;
    }
}
```

## App Scope Mapping

The loader should parse pure Helix theme files first. Do not require app-specific scopes for the initial implementation.

After the loader exists, map existing UI structs from Helix scopes:

Dialog suggested mapping:

```text
DialogTheme.background    <- ui.background
DialogTheme.border        <- ui.border
DialogTheme.border_active <- ui.border.focus or ui.text.focus
DialogTheme.title         <- ui.text.focus
DialogTheme.text          <- ui.text
DialogTheme.button        <- ui.text
DialogTheme.button_active <- ui.text.focus
```

Picker suggested mapping:

```text
PickerTheme.background <- ui.background
PickerTheme.foreground <- ui.text
PickerTheme.secondary  <- ui.text.info or ui.text
PickerTheme.border     <- ui.border
```

Do this as separate conversion helpers later, not inside the loader:

```rust
impl DialogTheme {
    pub fn from_helix_theme(theme: &Theme) -> Self;
}

impl PickerTheme {
    pub fn from_helix_theme(theme: &Theme) -> Self;
}
```

## Tests To Add

Add unit tests inside `src/theme.rs`.

Required tests:

1. Parses a string style:

```toml
"ui.text" = "red"
```

Expected:

```rust
theme.get("ui.text").fg == Some(Color::Red)
```

2. Parses `fg`, `bg`, and modifiers:

```toml
"ui.text.focus" = { fg = "#ffffff", bg = "0", modifiers = ["bold", "italic"] }
```

Expected:

```rust
fg == Some(Color::Rgb(255, 255, 255))
bg == Some(Color::Indexed(0))
add_modifier contains BOLD and ITALIC
```

3. Parses palette references:

```toml
"ui.text" = { fg = "text" }
[palette]
text = "#cdd6f4"
```

Expected `Color::Rgb(205, 214, 244)`.

4. Dot fallback:

```toml
"ui.text" = "green"
```

`theme.get("ui.text.focus")` returns the `ui.text` style.

5. Inheritance:

Parent has `ui.text` and palette color.
Child inherits parent and overrides one palette entry.
Expected child uses inherited style with overridden child palette color after merge.

6. Cycle detection:

`a.toml` inherits `b`, `b.toml` inherits `a`.
Expected `ThemeError::InheritanceCycle`.

7. Unknown style key errors:

```toml
"ui.text" = { nope = "red" }
```

Expected `ThemeError::InvalidStyle`.

## Test Command

Ask the user before running tests if the environment has issues. If tests are run, use:

```bash
cargo test --features tui theme
```

Do not run `cargo fmt`.

## Non-Goals For First Pass

Do not implement syntax highlighting integration.

Do not implement Helix runtime directory discovery.

Do not copy built-in Helix themes into this repository.

Do not support adaptive light/dark theme config yet.

Do not make app-specific `tui-pages.*` scopes required.

Do not silently ignore malformed theme entries.

## Implementation Order

1. Add `src/theme.rs` with error type, `Theme`, and `ThemeLoader`.
2. Implement `read_to_string` and `toml::from_str::<toml::Value>`.
3. Implement theme file discovery by name.
4. Implement inheritance loading and cycle detection.
5. Implement parent/child TOML merge, including special `[palette]` merge.
6. Implement built-in palette map.
7. Implement hex, ANSI index, and palette-name color parsing.
8. Implement style parsing.
9. Implement scope lookup with dot fallback.
10. Export the module from `src/lib.rs` behind `#[cfg(feature = "tui")]`.
11. Add unit tests listed above.
12. Run `cargo test --features tui theme` only if test execution is allowed.

