//! Picker content, fuzzy-search engine, and the scope-aware text-input
//! provider.
//!
//! [`PickerData`] is the turnkey state for a fuzzy-search overlay: a list of
//! [`PickerEntry`] rows, a canvas text input with scope (`%token`) parsing and
//! autocompletion, and a [`nucleo`]-backed ranking engine. It is generic over an
//! application-owned mode payload `M` (see [`PickerData::set_mode`]) that the
//! library stores but never inspects — mirroring how
//! [`DialogData`](crate::dialog::DialogData) carries an opaque purpose.

use crate::canvas::{
    DataProvider, SuggestionItem, SuggestionQuery, SuggestionTrigger, TextInputDataProvider,
    TextInputEventOutcome, TextInputState,
};
use super::query::{PickerCommandArgument, PickerCommandQuery, PickerCommandSpec};
use nucleo::{
    Config as NucleoConfig, Matcher, Utf32String,
    pattern::{CaseMatching, Normalization, Pattern},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerField {
    pub key: String,
    pub value: String,
}

impl PickerField {
    pub fn new<K: Into<String>, V: Into<String>>(key: K, value: V) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerScope {
    pub token: String,
    pub aliases: Vec<String>,
    pub label: String,
    pub field_keys: Vec<String>,
    pub completion_key: Option<String>,
    pub value_token_limit: Option<usize>,
    pub command_argument: PickerCommandArgument,
    /// When set, the inline completion suffix is suppressed while the cursor is
    /// editing this scope's value. Use it for scopes whose value is free-form
    /// (e.g. a row search) rather than a known candidate to complete toward.
    pub suppress_value_completion: bool,
}

impl PickerScope {
    pub fn new<T: Into<String>, L: Into<String>>(
        token: T,
        label: L,
        field_keys: Vec<String>,
    ) -> Self {
        Self {
            token: token.into(),
            aliases: Vec::new(),
            label: label.into(),
            field_keys,
            completion_key: None,
            value_token_limit: None,
            command_argument: PickerCommandArgument::None,
            suppress_value_completion: false,
        }
    }

    pub fn with_aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases = aliases.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_completion_key<S: Into<String>>(mut self, key: S) -> Self {
        self.completion_key = Some(key.into());
        self
    }

    pub fn with_value_token_limit(mut self, limit: usize) -> Self {
        self.value_token_limit = Some(limit);
        if limit == 1 {
            self.command_argument = PickerCommandArgument::Required;
        }
        self
    }

    pub fn with_command_argument(mut self) -> Self {
        self.command_argument = PickerCommandArgument::Required;
        self.value_token_limit = Some(1);
        self
    }

    pub fn without_command_argument(mut self) -> Self {
        self.command_argument = PickerCommandArgument::None;
        self.value_token_limit = None;
        self
    }

    /// Suppress the inline completion suffix while editing this scope's value.
    pub fn with_suppressed_value_completion(mut self) -> Self {
        self.suppress_value_completion = true;
        self
    }

    fn matches_token(&self, token: &str) -> bool {
        self.token == token || self.aliases.iter().any(|alias| alias == token)
    }

    fn completion_value<'a>(&'a self, entry: &'a PickerEntry) -> Option<&'a str> {
        if let Some(key) = &self.completion_key {
            if key == "label" {
                return Some(entry.label.as_str());
            }

            if let Some(field) = entry.fields.iter().find(|field| &field.key == key) {
                return Some(field.value.as_str());
            }
        }

        for key in &self.field_keys {
            if key == "label" {
                return Some(entry.label.as_str());
            }

            if let Some(field) = entry.fields.iter().find(|field| &field.key == key) {
                return Some(field.value.as_str());
            }
        }

        Some(entry.label.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickerEntry {
    pub label: String,
    pub preview_lines: Vec<String>,
    pub fields: Vec<PickerField>,
}

impl PickerEntry {
    pub fn new<L: Into<String>>(label: L, preview_lines: Vec<String>) -> Self {
        let label = label.into();
        Self {
            fields: vec![PickerField::new("label", label.clone())],
            label,
            preview_lines,
        }
    }

    pub fn with_fields(mut self, fields: Vec<PickerField>) -> Self {
        self.fields.extend(fields);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerLayout {
    #[default]
    Responsive,
    SinglePane,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerHead {
    pub columns: Vec<PickerHeadColumn>,
}

impl Default for PickerHead {
    fn default() -> Self {
        Self::new()
    }
}

impl PickerHead {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
        }
    }

    pub fn with_columns(columns: Vec<PickerHeadColumn>) -> Self {
        Self { columns }
    }

    pub fn field<L: Into<String>, K: Into<String>>(
        self,
        label: L,
        field_key: K,
    ) -> PickerHeadColumnBuilder {
        PickerHeadColumnBuilder {
            head: self,
            label: label.into(),
            field_key: field_key.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerHeadColumnBuilder {
    head: PickerHead,
    label: String,
    field_key: String,
}

impl PickerHeadColumnBuilder {
    pub fn width(mut self, width: usize) -> PickerHead {
        self.head
            .columns
            .push(PickerHeadColumn::new(self.label, self.field_key, width));
        self.head
    }

    pub fn flex(self) -> PickerHead {
        self.width(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerHeadColumn {
    pub label: String,
    pub field_key: String,
    pub width: usize,
}

impl PickerHeadColumn {
    pub fn new<L: Into<String>, K: Into<String>>(label: L, field_key: K, width: usize) -> Self {
        Self {
            label: label.into(),
            field_key: field_key.into(),
            width,
        }
    }
}

/// Per-field-key scoring weights for the fuzzy matcher.
///
/// A higher weight makes matches in that field rank above matches in other
/// fields. Every entry's `label` field is always present, so the default keeps
/// `label` heaviest; applications layer their own domain keys on top via
/// [`PickerFieldWeights::with_override`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerFieldWeights {
    default: u32,
    overrides: Vec<(String, u32)>,
}

impl Default for PickerFieldWeights {
    fn default() -> Self {
        Self {
            default: 10,
            overrides: vec![("label".to_string(), 16)],
        }
    }
}

impl PickerFieldWeights {
    /// Weights with a single fallback applied to every field key.
    pub fn uniform(weight: u32) -> Self {
        Self {
            default: weight,
            overrides: Vec::new(),
        }
    }

    pub fn with_default(mut self, weight: u32) -> Self {
        self.default = weight;
        self
    }

    pub fn with_override<K: Into<String>>(mut self, key: K, weight: u32) -> Self {
        let key = key.into();
        self.overrides.retain(|(existing, _)| existing != &key);
        self.overrides.push((key, weight));
        self
    }

    pub fn weight(&self, key: &str) -> u32 {
        self.overrides
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, weight)| *weight)
            .unwrap_or(self.default)
    }
}

/// A generic, scope-aware fuzzy picker.
///
/// `M` is an application-owned mode payload stored opaquely (see
/// [`set_mode`](PickerData::set_mode)); the library never inspects it. Use
/// `PickerData<()>` (the default) when you don't need one.
pub struct PickerData<M = ()> {
    pub title: Option<String>,
    pub input: TextInputState<PickerInputProvider>,
    pub query_placeholder: String,
    pub source_entries: Vec<PickerEntry>,
    pub filtered_indices: Vec<usize>,
    pub selected_filtered_index: Option<usize>,
    pub empty_preview: String,
    pub scopes: Vec<PickerScope>,
    pub layout: PickerLayout,
    pub head: Option<PickerHead>,
    mode: Option<M>,
    field_weights: PickerFieldWeights,
    implicit_scopes: Vec<PickerImplicitScope>,
    search_cache: Vec<PickerEntrySearchCache>,
    matcher: Matcher,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickerInputProvider {
    value: String,
    scopes: Vec<PickerScope>,
    suggestion_scopes: Option<Vec<PickerScope>>,
}

impl PickerInputProvider {
    fn set_scopes(&mut self, scopes: Vec<PickerScope>) {
        self.scopes = scopes;
        self.suggestion_scopes = None;
    }

    fn set_suggestion_scopes(&mut self, scopes: Option<Vec<PickerScope>>) {
        self.suggestion_scopes = scopes;
    }

    fn active_suggestion_scopes(&self) -> &[PickerScope] {
        self.suggestion_scopes
            .as_deref()
            .unwrap_or(self.scopes.as_slice())
    }

    fn scope_query(&self, cursor_char: usize) -> Option<SuggestionQuery> {
        let chars = self.value.chars().collect::<Vec<_>>();
        let cursor = cursor_char.min(chars.len());
        let mut start = cursor;
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }

        let mut end = cursor;
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }

        let typed = chars[start..cursor].iter().collect::<String>();
        typed
            .starts_with('%')
            .then(|| SuggestionQuery::with_replace_range(typed, (start, end)))
    }
}

impl DataProvider for PickerInputProvider {
    fn field_count(&self) -> usize {
        1
    }

    fn field_name(&self, _index: usize) -> &str {
        "Search"
    }

    fn field_value(&self, index: usize) -> &str {
        if index == 0 { &self.value } else { "" }
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        if index == 0 {
            self.value = sanitize_picker_input(value);
        }
    }

    fn supports_suggestions(&self, field_index: usize) -> bool {
        field_index == 0 && !self.active_suggestion_scopes().is_empty()
    }

    fn suggestion_trigger(&self, field_index: usize) -> SuggestionTrigger {
        if self.supports_suggestions(field_index) {
            SuggestionTrigger::SpecialChar('%')
        } else {
            SuggestionTrigger::None
        }
    }

    fn suggestion_query(&self, _field_index: usize, cursor_char: usize) -> Option<SuggestionQuery> {
        self.scope_query(cursor_char)
    }

    fn fetch_suggestions_sync(&self, _field_index: usize, query: &str) -> Vec<SuggestionItem> {
        let query = query.to_lowercase();
        self.active_suggestion_scopes()
            .iter()
            .filter_map(|scope| {
                let token = format!("%{}", scope.token);
                let token_matches = token.starts_with(&query);
                let alias_matches = scope
                    .aliases
                    .iter()
                    .map(|alias| format!("%{}", alias))
                    .any(|alias| alias.starts_with(&query));
                (token_matches || alias_matches).then(|| {
                    SuggestionItem::new(
                        format!("{}  {}", token, scope.label),
                        format!("{} ", token),
                    )
                })
            })
            .collect()
    }

    fn accept_suggestion(
        &mut self,
        field_index: usize,
        _cursor_char: usize,
        suggestion: &SuggestionItem,
        query: &SuggestionQuery,
    ) -> usize {
        if field_index != 0 {
            return 0;
        }

        let replacement = suggestion.value_to_store.as_str();
        let (start, end) = query
            .replace_range
            .unwrap_or((0, self.value.chars().count()));
        let chars = self.value.chars().collect::<Vec<_>>();
        let start = start.min(chars.len());
        let end = end.min(chars.len()).max(start);

        let mut updated = String::new();
        updated.extend(chars[..start].iter());
        updated.push_str(replacement);
        updated.extend(chars[end..].iter());
        self.value = sanitize_picker_input(updated);
        start + replacement.chars().count()
    }
}

impl TextInputDataProvider for PickerInputProvider {
    fn from_text(text: String) -> Self {
        Self {
            value: sanitize_picker_input(text),
            scopes: Vec::new(),
            suggestion_scopes: None,
        }
    }

    fn to_text(&self) -> String {
        self.value.clone()
    }

    fn set_text(&mut self, text: String) {
        self.value = sanitize_picker_input(text);
    }
}

fn sanitize_picker_input(text: String) -> String {
    text.chars()
        .take_while(|&ch| ch != '\n' && ch != '\r')
        .collect()
}

// Experimental picker-query whitespace normalization. Keep this isolated so the
// behavior can be removed if command-query editing should return to raw input.
fn normalize_picker_input_whitespace_experimental(text: &str, cursor: usize) -> (String, usize) {
    let normalized = normalize_picker_input_text_whitespace_experimental(text);
    let prefix = text.chars().take(cursor).collect::<String>();
    let normalized_cursor = normalize_picker_input_text_whitespace_experimental(&prefix)
        .chars()
        .count()
        .min(normalized.chars().count());

    (normalized, normalized_cursor)
}

fn normalize_picker_input_text_whitespace_experimental(text: &str) -> String {
    let mut normalized = String::new();
    let mut pending_separator = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if !normalized.is_empty() {
                pending_separator = true;
            }
            continue;
        }

        if pending_separator {
            normalized.push(' ');
            pending_separator = false;
        }
        normalized.push(ch);
    }

    if pending_separator && !normalized.is_empty() {
        normalized.push(' ');
    }

    normalized
}

impl<M> PickerData<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_entries(entries: Vec<PickerEntry>) -> Self {
        let mut picker = Self {
            source_entries: entries,
            ..Self::default()
        };
        picker.rebuild_search_cache();
        picker.refresh_filter();
        picker
    }

    pub fn with_layout(mut self, layout: PickerLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn single_pane(mut self) -> Self {
        self.layout = PickerLayout::SinglePane;
        self
    }

    pub fn set_layout(&mut self, layout: PickerLayout) {
        self.layout = layout;
    }

    pub fn with_head(mut self, head: PickerHead) -> Self {
        self.head = Some(head);
        self
    }

    pub fn set_head(&mut self, head: PickerHead) {
        self.head = Some(head);
    }

    pub fn clear_head(&mut self) {
        self.head = None;
    }

    pub fn with_field_weights(mut self, weights: PickerFieldWeights) -> Self {
        self.field_weights = weights;
        self.refresh_filter();
        self
    }

    pub fn set_field_weights(&mut self, weights: PickerFieldWeights) {
        self.field_weights = weights;
        self.refresh_filter();
    }

    pub fn field_weights(&self) -> &PickerFieldWeights {
        &self.field_weights
    }

    /// Attach an application-owned mode payload. The library stores it verbatim
    /// and never inspects it; read it back with [`mode_ref`](PickerData::mode_ref)
    /// / [`mode_mut`](PickerData::mode_mut).
    pub fn set_mode(&mut self, mode: M) {
        self.mode = Some(mode);
    }

    pub fn with_mode(mut self, mode: M) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn clear_mode(&mut self) {
        self.mode = None;
    }

    pub fn take_mode(&mut self) -> Option<M> {
        self.mode.take()
    }

    pub fn mode_ref(&self) -> Option<&M> {
        self.mode.as_ref()
    }

    pub fn mode_mut(&mut self) -> Option<&mut M> {
        self.mode.as_mut()
    }

    pub fn set_entries(&mut self, entries: Vec<PickerEntry>) {
        if self.source_entries == entries {
            return;
        }

        self.source_entries = entries;
        self.rebuild_search_cache();
        self.refresh_filter();
    }

    pub fn set_scopes(&mut self, scopes: Vec<PickerScope>) {
        if self.scopes == scopes {
            return;
        }

        self.scopes = scopes;
        self.input
            .data_provider_mut()
            .set_scopes(self.scopes.clone());
        self.refresh_filter();
    }

    pub fn set_suggestion_scopes(&mut self, scopes: Option<Vec<PickerScope>>) {
        self.input.data_provider_mut().set_suggestion_scopes(scopes);
        self.input.check_suggestion_trigger();
    }

    pub fn clear_suggestion_scopes(&mut self) {
        self.set_suggestion_scopes(None);
    }

    pub fn set_implicit_scope(&mut self, token: &str, value: &str) {
        self.implicit_scopes.retain(|scope| scope.token != token);
        if !value.trim().is_empty() {
            self.implicit_scopes
                .push(PickerImplicitScope::new(token, value));
        }
        self.refresh_filter();
    }

    pub fn set_selected_source_index(&mut self, source_index: Option<usize>) {
        self.selected_filtered_index = source_index.and_then(|source_index| {
            self.filtered_indices
                .iter()
                .position(|candidate| *candidate == source_index)
        });

        self.apply_selection_change();
    }

    pub fn push_char(&mut self, ch: char) {
        self.input.enter_edit_mode();
        let _ = self.input.insert_char(ch);
        self.refresh_filter();
    }

    pub fn pop_char(&mut self) {
        let _ = self.input.delete_backward();
        self.refresh_filter();
    }

    pub fn clear_input(&mut self) {
        self.input.set_text("");
        self.input.enter_edit_mode();
        self.refresh_filter();
    }

    pub fn input_ref(&self) -> &TextInputState<PickerInputProvider> {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut TextInputState<PickerInputProvider> {
        &mut self.input
    }

    pub fn refresh_filter(&mut self) {
        self.normalize_input_whitespace_experimental();
        self.apply_query_change();
    }

    pub fn move_up(&mut self) {
        if self.input.is_suggestions_active() {
            self.input.suggestions_prev();
            return;
        }

        if let Some(index) = self.selected_filtered_index {
            if index > 0 {
                self.selected_filtered_index = Some(index - 1);
            }
        } else if !self.filtered_indices.is_empty() {
            self.selected_filtered_index = Some(0);
        }

        self.apply_selection_change();
    }

    pub fn move_down(&mut self) {
        if self.input.is_suggestions_active() {
            self.input.suggestions_next();
            return;
        }

        if let Some(index) = self.selected_filtered_index {
            if index + 1 < self.filtered_indices.len() {
                self.selected_filtered_index = Some(index + 1);
            }
        } else if !self.filtered_indices.is_empty() {
            self.selected_filtered_index = Some(0);
        }

        self.apply_selection_change();
    }

    pub fn filtered_len(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn visible_entries(&self) -> impl Iterator<Item = (usize, &PickerEntry)> {
        self.filtered_indices
            .iter()
            .enumerate()
            .filter_map(|(filtered_index, source_index)| {
                self.source_entries
                    .get(*source_index)
                    .map(|entry| (filtered_index, entry))
            })
    }

    pub fn selected_entry(&self) -> Option<&PickerEntry> {
        self.selected_filtered_index
            .and_then(|filtered_index| self.filtered_indices.get(filtered_index))
            .and_then(|source_index| self.source_entries.get(*source_index))
    }

    pub fn count_label(&self) -> String {
        match self.selected_filtered_index {
            Some(index) if !self.filtered_indices.is_empty() => {
                format!("{}/{}", index + 1, self.filtered_indices.len())
            }
            _ => format!("0/{}", self.filtered_indices.len()),
        }
    }

    pub fn scope_hint(&self) -> String {
        if self.scopes.is_empty() {
            return String::new();
        }

        self.scopes
            .iter()
            .map(|scope| format!("%{}", scope.token))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn active_scope_label(&self) -> Option<String> {
        ParsedQuery::parse(&self.input.text(), &self.scopes)
            .active_scope_label(self.input.cursor_position())
            .map(str::to_string)
    }

    pub fn paste(&mut self, text: &str) -> TextInputEventOutcome {
        let outcome = self.input.paste(text);
        if matches!(outcome, TextInputEventOutcome::Handled) {
            self.refresh_filter();
        }
        outcome
    }

    pub fn autocomplete_selected(&mut self) -> TextInputEventOutcome {
        if self.input.is_suggestions_active() {
            if self.input.apply_suggestion().is_some() {
                self.refresh_filter();
                return TextInputEventOutcome::Handled;
            }
        }

        let outcome = self.input.accept_suggestion_suffix();
        if matches!(outcome, TextInputEventOutcome::Handled) {
            self.refresh_filter();
        }
        outcome
    }

    fn apply_query_change(&mut self) {
        self.rebuild_filtered_indices();
        self.normalize_selection();
        self.recompute_inline_suggestion();
        self.input.check_suggestion_trigger();
    }

    fn apply_selection_change(&mut self) {
        self.normalize_selection();
        self.recompute_inline_suggestion();
    }

    fn normalize_input_whitespace_experimental(&mut self) {
        let text = self.input.text();
        let cursor = self.input.cursor_position();
        let (normalized, normalized_cursor) =
            normalize_picker_input_whitespace_experimental(&text, cursor);
        if normalized != text {
            self.input.set_text(normalized);
            self.input.set_cursor_position(normalized_cursor);
        }
    }

    fn rebuild_filtered_indices(&mut self) {
        let query = self.input.text();
        let parsed_query = ParsedQuery::parse(&query, &self.scopes)
            .with_implicit_scopes(&self.implicit_scopes, &self.scopes);
        let weights = &self.field_weights;
        let mut matches = self
            .source_entries
            .iter()
            .enumerate()
            .filter_map(|(index, _)| {
                let cache = self.search_cache.get(index)?;
                let score = parsed_query.score_entry(cache, &mut self.matcher, weights)?;
                Some(PickerRankedEntry {
                    source_index: index,
                    score,
                })
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.source_index.cmp(&right.source_index))
        });
        self.filtered_indices = matches
            .into_iter()
            .map(|entry| entry.source_index)
            .collect();
    }

    fn normalize_selection(&mut self) {
        self.selected_filtered_index = match self.selected_filtered_index {
            Some(index) if index < self.filtered_indices.len() => Some(index),
            _ if self.filtered_indices.is_empty() => None,
            _ => Some(0),
        };
    }

    fn recompute_inline_suggestion(&mut self) {
        let query = self.input.text();
        let cursor = self.input.cursor_position();
        let parsed = ParsedQuery::parse(&query, &self.scopes);
        let suffix = self
            .selected_entry()
            .and_then(|entry| parsed.trailing_completion_suffix(&query, cursor, entry));

        if let Some(suffix) = suffix {
            self.input.set_suggestion_suffix(suffix);
        } else {
            self.input.clear_suggestion_suffix();
        }
    }

    fn rebuild_search_cache(&mut self) {
        self.search_cache = self
            .source_entries
            .iter()
            .map(PickerEntrySearchCache::from_entry)
            .collect();
    }
}

impl<M> Default for PickerData<M> {
    fn default() -> Self {
        let mut input = TextInputState::<PickerInputProvider>::default();
        input.enter_edit_mode();
        Self {
            title: None,
            input,
            query_placeholder: String::new(),
            source_entries: Vec::new(),
            filtered_indices: Vec::new(),
            selected_filtered_index: None,
            empty_preview: String::new(),
            scopes: Vec::new(),
            layout: PickerLayout::Responsive,
            head: None,
            mode: None,
            field_weights: PickerFieldWeights::default(),
            implicit_scopes: Vec::new(),
            search_cache: Vec::new(),
            matcher: build_picker_matcher(),
        }
    }
}

impl<M: Clone> Clone for PickerData<M> {
    fn clone(&self) -> Self {
        let mut input = TextInputState::<PickerInputProvider>::default();
        input.set_text(self.input.text());
        input.data_provider_mut().set_scopes(self.scopes.clone());
        input.enter_edit_mode();
        let mut picker = Self {
            title: self.title.clone(),
            input,
            query_placeholder: self.query_placeholder.clone(),
            source_entries: self.source_entries.clone(),
            filtered_indices: self.filtered_indices.clone(),
            selected_filtered_index: self.selected_filtered_index,
            empty_preview: self.empty_preview.clone(),
            scopes: self.scopes.clone(),
            layout: self.layout,
            head: self.head.clone(),
            mode: self.mode.clone(),
            field_weights: self.field_weights.clone(),
            implicit_scopes: self.implicit_scopes.clone(),
            search_cache: Vec::new(),
            matcher: build_picker_matcher(),
        };
        picker.rebuild_search_cache();
        picker.recompute_inline_suggestion();
        picker
    }
}

impl<M: PartialEq> PartialEq for PickerData<M> {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.input.text() == other.input.text()
            && self.query_placeholder == other.query_placeholder
            && self.source_entries == other.source_entries
            && self.filtered_indices == other.filtered_indices
            && self.selected_filtered_index == other.selected_filtered_index
            && self.empty_preview == other.empty_preview
            && self.scopes == other.scopes
            && self.layout == other.layout
            && self.head == other.head
            && self.mode == other.mode
            && self.field_weights == other.field_weights
            && self.implicit_scopes == other.implicit_scopes
    }
}

impl<M: Eq> Eq for PickerData<M> {}

impl<M: std::fmt::Debug> std::fmt::Debug for PickerData<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PickerData")
            .field("title", &self.title)
            .field("input_text", &self.input.text())
            .field("query_placeholder", &self.query_placeholder)
            .field("source_entries", &self.source_entries)
            .field("filtered_indices", &self.filtered_indices)
            .field("selected_filtered_index", &self.selected_filtered_index)
            .field("empty_preview", &self.empty_preview)
            .field("scopes", &self.scopes)
            .field("layout", &self.layout)
            .field("head", &self.head)
            .field("mode", &self.mode)
            .field("field_weights", &self.field_weights)
            .field("implicit_scopes", &self.implicit_scopes)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct QueryTerm {
    text: String,
    pattern: Pattern,
}

#[derive(Debug, Clone)]
enum QuerySegmentKind<'a> {
    Default,
    Scoped {
        scope: &'a PickerScope,
        scope_token_char_end: usize,
    },
}

#[derive(Debug, Clone)]
struct QuerySegment<'a> {
    kind: QuerySegmentKind<'a>,
    text: String,
    terms: Vec<QueryTerm>,
}

#[derive(Debug, Clone)]
struct PickerCachedField {
    key: String,
    value: String,
    haystack: Utf32String,
}

#[derive(Debug, Clone, Default)]
struct PickerEntrySearchCache {
    fields: Vec<PickerCachedField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PickerRankedEntry {
    source_index: usize,
    score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PickerImplicitScope {
    token: String,
    value: String,
}

impl PickerImplicitScope {
    fn new(token: &str, value: &str) -> Self {
        Self {
            token: token.to_string(),
            value: value.trim().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedQuery<'a> {
    query_char_len: usize,
    ends_with_whitespace: bool,
    segments: Vec<QuerySegment<'a>>,
}

impl<'a> ParsedQuery<'a> {
    fn parse(query: &'a str, scopes: &'a [PickerScope]) -> Self {
        let mut segments = Vec::new();
        let query_char_len = query.chars().count();
        let command_specs = command_specs_for_scopes(scopes);
        let parsed = PickerCommandQuery::parse_with_specs(query, &command_specs);

        for clause in parsed.clauses() {
            let Some(command) = clause.command() else {
                push_default_segment(&mut segments, clause.selection_query_text());
                continue;
            };

            let Some(scope) = scopes.iter().find(|scope| scope.matches_token(command)) else {
                let mut text = format!("%{}", command);
                let terms = clause.terms();
                if !terms.is_empty() {
                    text.push(' ');
                    text.push_str(&terms.join(" "));
                }
                push_default_segment(&mut segments, text);
                continue;
            };

            let scoped_text = match scope.command_argument {
                PickerCommandArgument::Required => clause.argument_or_empty().to_string(),
                PickerCommandArgument::None => clause.selection_query_text(),
            };
            segments.push(QuerySegment::from_scoped_text(scope, scoped_text));

            if scope.command_argument == PickerCommandArgument::Required {
                push_default_segment(&mut segments, clause.selection_query_text());
            }
        }

        Self {
            query_char_len,
            ends_with_whitespace: query.chars().last().is_some_and(char::is_whitespace),
            segments,
        }
    }

    fn with_implicit_scopes(
        mut self,
        implicit_scopes: &[PickerImplicitScope],
        scopes: &'a [PickerScope],
    ) -> Self {
        for implicit_scope in implicit_scopes {
            if self.has_scope(&implicit_scope.token) {
                continue;
            }

            let Some(scope) = scopes
                .iter()
                .find(|scope| scope.matches_token(&implicit_scope.token))
            else {
                continue;
            };

            self.segments.push(QuerySegment::from_scoped_value(
                scope,
                implicit_scope.value.clone(),
            ));
        }

        self
    }

    fn has_scope(&self, token: &str) -> bool {
        self.segments.iter().any(|segment| match &segment.kind {
            QuerySegmentKind::Scoped { scope, .. } => scope.matches_token(token),
            QuerySegmentKind::Default => false,
        })
    }

    fn active_scope_label(&self, cursor_chars: usize) -> Option<&'a str> {
        let QuerySegmentKind::Scoped { scope, .. } =
            self.trailing_active_segment(cursor_chars)?.kind
        else {
            return None;
        };

        Some(scope.label.as_str())
    }

    fn trailing_completion_suffix(
        &self,
        _query: &str,
        cursor_chars: usize,
        entry: &PickerEntry,
    ) -> Option<String> {
        // A scope can opt out of inline value completion (e.g. a free-form row
        // search). Suppress the suffix while the cursor is editing such a
        // scope's non-empty value, including a trailing space after it.
        if cursor_chars == self.query_char_len {
            if let Some(QuerySegment {
                kind: QuerySegmentKind::Scoped { scope, .. },
                text,
                ..
            }) = self.segments.last()
            {
                if scope.suppress_value_completion && !text.is_empty() {
                    return None;
                }
            }
        }

        let Some(segment) = self.trailing_active_segment(cursor_chars) else {
            if cursor_chars == self.query_char_len
                && self.ends_with_whitespace
                && self
                    .segments
                    .iter()
                    .any(|segment| matches!(segment.kind, QuerySegmentKind::Scoped { .. }))
                && !entry.label.is_empty()
            {
                return Some(entry.label.clone());
            }

            return None;
        };
        let candidate = match &segment.kind {
            QuerySegmentKind::Default => entry.label.as_str(),
            QuerySegmentKind::Scoped { scope, .. } => scope.completion_value(entry)?,
        };

        let typed = segment.text.as_str();
        if typed.is_empty() {
            return match &segment.kind {
                QuerySegmentKind::Scoped {
                    scope_token_char_end,
                    ..
                } if *scope_token_char_end == self.query_char_len => {
                    Some(format!(" {}", candidate))
                }
                _ if candidate.is_empty() => None,
                _ => Some(candidate.to_string()),
            };
        }

        let Some(suffix_start) = case_insensitive_prefix_boundary(candidate, typed) else {
            return None;
        };
        let suffix = candidate[suffix_start..].to_string();
        if suffix.is_empty() {
            return None;
        }

        Some(suffix)
    }

    fn trailing_active_segment(&self, cursor_chars: usize) -> Option<&QuerySegment<'a>> {
        if cursor_chars != self.query_char_len {
            return None;
        }

        let segment = self.segments.last()?;
        if !self.ends_with_whitespace {
            return Some(segment);
        }

        match &segment.kind {
            QuerySegmentKind::Scoped { .. } if segment.text.is_empty() => Some(segment),
            QuerySegmentKind::Default | QuerySegmentKind::Scoped { .. } => None,
        }
    }

    fn score_entry(
        &self,
        entry_cache: &PickerEntrySearchCache,
        matcher: &mut Matcher,
        weights: &PickerFieldWeights,
    ) -> Option<u32> {
        if self.segments.is_empty() {
            return Some(0);
        }

        let mut total_score = 0;
        for segment in &self.segments {
            total_score += segment.score(entry_cache, matcher, weights)?;
        }

        Some(total_score)
    }
}

impl<'a> QuerySegment<'a> {
    fn from_text(kind: QuerySegmentKind<'a>, text: String) -> Self {
        Self {
            kind,
            terms: split_terms(&text),
            text,
        }
    }

    fn from_scoped_text(scope: &'a PickerScope, text: String) -> Self {
        Self::from_text(
            QuerySegmentKind::Scoped {
                scope,
                scope_token_char_end: 0,
            },
            text,
        )
    }

    fn from_scoped_value(scope: &'a PickerScope, text: String) -> Self {
        Self::from_scoped_text(scope, text)
    }
}

fn push_default_segment<'a>(segments: &mut Vec<QuerySegment<'a>>, text: String) {
    if !text.trim().is_empty() {
        segments.push(QuerySegment::from_text(
            QuerySegmentKind::Default,
            text.trim().to_string(),
        ));
    }
}

fn command_specs_for_scopes(scopes: &[PickerScope]) -> Vec<PickerCommandSpec> {
    scopes
        .iter()
        .map(|scope| {
            PickerCommandSpec::new(scope.token.clone(), scope.command_argument)
                .with_aliases(scope.aliases.clone())
        })
        .collect()
}

impl QuerySegment<'_> {
    fn score(
        &self,
        entry_cache: &PickerEntrySearchCache,
        matcher: &mut Matcher,
        weights: &PickerFieldWeights,
    ) -> Option<u32> {
        if self.terms.is_empty() {
            return match &self.kind {
                QuerySegmentKind::Default => Some(0),
                QuerySegmentKind::Scoped { scope, .. } => entry_cache
                    .fields
                    .iter()
                    .any(|field| {
                        !field.value.trim().is_empty()
                            && scope.field_keys.iter().any(|key| key == &field.key)
                    })
                    .then_some(0),
            };
        }

        let mut segment_score = 0;
        for term in &self.terms {
            let best = match &self.kind {
                QuerySegmentKind::Default => {
                    term.best_score(entry_cache.fields.iter(), matcher, weights)
                }
                QuerySegmentKind::Scoped { scope, .. } => {
                    let scoped_fields = entry_cache
                        .fields
                        .iter()
                        .filter(|field| scope.field_keys.iter().any(|key| key == &field.key))
                        .collect::<Vec<_>>();

                    if scoped_fields.is_empty() {
                        term.best_score(
                            entry_cache
                                .fields
                                .iter()
                                .filter(|field| field.key == "label"),
                            matcher,
                            weights,
                        )
                    } else {
                        term.best_score(scoped_fields.into_iter(), matcher, weights)
                    }
                }
            }?;

            segment_score += best;
        }

        Some(segment_score)
    }
}

impl QueryTerm {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            pattern: Pattern::parse(text, CaseMatching::Smart, Normalization::Smart),
        }
    }

    fn best_score<'a, I>(
        &self,
        candidates: I,
        matcher: &mut Matcher,
        weights: &PickerFieldWeights,
    ) -> Option<u32>
    where
        I: IntoIterator<Item = &'a PickerCachedField>,
    {
        let mut best = None;

        for candidate in candidates {
            let Some(base_score) = self.pattern.score(candidate.haystack.slice(..), matcher) else {
                continue;
            };

            let score = base_score.saturating_mul(weights.weight(&candidate.key))
                + field_match_bonus(&self.text, &candidate.value);
            best = Some(best.map_or(score, |current: u32| current.max(score)));
        }

        best
    }
}

impl PickerEntrySearchCache {
    fn from_entry(entry: &PickerEntry) -> Self {
        Self {
            fields: entry
                .fields
                .iter()
                .map(|field| PickerCachedField {
                    key: field.key.clone(),
                    value: field.value.clone(),
                    haystack: Utf32String::from(field.value.as_str()),
                })
                .collect(),
        }
    }
}

fn split_terms(text: &str) -> Vec<QueryTerm> {
    text.split_whitespace().map(QueryTerm::new).collect()
}

fn build_picker_matcher() -> Matcher {
    let mut config = NucleoConfig::DEFAULT;
    config.prefer_prefix = true;
    Matcher::new(config)
}

fn field_match_bonus(query: &str, value: &str) -> u32 {
    let needle = query.trim();
    if needle.is_empty() {
        return 0;
    }

    let needle_lower = needle.to_lowercase();
    let value_lower = value.to_lowercase();

    if value_lower == needle_lower {
        2_000
    } else if value_lower.starts_with(&needle_lower) {
        900
    } else if value_lower
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|part| !part.is_empty() && part.starts_with(&needle_lower))
    {
        450
    } else if value_lower.contains(&needle_lower) {
        150
    } else {
        0
    }
}

fn case_insensitive_prefix_boundary(candidate: &str, typed: &str) -> Option<usize> {
    let typed_lower = typed.to_lowercase();

    let mut candidate_prefix_lower = String::new();
    for (byte_index, ch) in candidate.char_indices() {
        candidate_prefix_lower.extend(ch.to_lowercase());
        let next_boundary = byte_index + ch.len_utf8();

        if candidate_prefix_lower.len() >= typed_lower.len() {
            return candidate_prefix_lower
                .starts_with(&typed_lower)
                .then_some(next_boundary);
        }

        if !typed_lower.starts_with(&candidate_prefix_lower) {
            return None;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_picker() -> PickerData {
        let entries = vec![
            PickerEntry::new("invoice", vec![]).with_fields(vec![
                PickerField::new("profile", "finance"),
                PickerField::new("dependency", "customer department"),
            ]),
            PickerEntry::new("customer", vec![]).with_fields(vec![
                PickerField::new("profile", "finance"),
                PickerField::new("dependency", ""),
            ]),
            PickerEntry::new("shipment", vec![]).with_fields(vec![
                PickerField::new("profile", "ops"),
                PickerField::new("dependency", "warehouse"),
            ]),
        ];

        let mut picker = PickerData::with_entries(entries);
        picker.set_scopes(vec![
            PickerScope::new("table", "Table", vec!["label".to_string()])
                .with_completion_key("label"),
            PickerScope::new("profile", "Profile", vec!["profile".to_string()])
                .with_completion_key("profile"),
            PickerScope::new("dependency", "Dependency", vec!["dependency".to_string()])
                .with_aliases(["dep"])
                .with_completion_key("dependency"),
        ]);
        picker
    }

    #[test]
    fn plain_query_filters_entries() {
        let mut picker = build_picker();
        picker.input.set_text("invoice");
        picker.refresh_filter();

        assert_eq!(picker.filtered_len(), 1);
        assert_eq!(
            picker.selected_entry().map(|entry| entry.label.as_str()),
            Some("invoice")
        );
    }

    #[test]
    fn scoped_query_filters_matching_field() {
        let mut picker = build_picker();
        picker.input.set_text("%dependency department");
        picker.refresh_filter();

        assert_eq!(picker.filtered_len(), 1);
        assert_eq!(
            picker.selected_entry().map(|entry| entry.label.as_str()),
            Some("invoice")
        );
    }

    #[test]
    fn repeated_scoped_clauses_filter_with_and_logic() {
        let mut picker = build_picker();
        picker
            .input
            .set_text("invoice %profile finance %dependency customer");
        picker.refresh_filter();

        assert_eq!(picker.filtered_len(), 1);
        assert_eq!(
            picker.selected_entry().map(|entry| entry.label.as_str()),
            Some("invoice")
        );
    }

    #[test]
    fn scope_aliases_are_supported() {
        let mut picker = build_picker();
        picker.input.set_text("%dep customer");
        picker.refresh_filter();

        assert_eq!(picker.filtered_len(), 1);
        assert_eq!(
            picker.selected_entry().map(|entry| entry.label.as_str()),
            Some("invoice")
        );
    }

    #[test]
    fn selects_matching_source_index_after_filtering() {
        let mut picker = build_picker();
        picker.set_selected_source_index(Some(1));

        assert_eq!(
            picker.selected_entry().map(|entry| entry.label.as_str()),
            Some("customer")
        );
    }

    #[test]
    fn selected_entry_exposes_inline_suffix_for_prefix_query() {
        let mut picker = build_picker();
        picker.input.set_text("inv");
        picker.refresh_filter();

        assert_eq!(picker.input.suggestion_suffix(), Some("oice"));
    }

    #[test]
    fn inline_suffix_uses_unicode_boundary_after_case_folded_prefix() {
        let entries = vec![PickerEntry::new("İstanbul", vec![])];
        let mut picker = PickerData::<()>::with_entries(entries);
        picker.input.set_text("i");
        picker.refresh_filter();

        assert_eq!(picker.input.suggestion_suffix(), Some("stanbul"));
    }

    #[test]
    fn autocomplete_selected_accepts_highlighted_suffix() {
        let mut picker = build_picker();
        picker.input.set_text("inv");
        picker.refresh_filter();

        let outcome = picker.autocomplete_selected();

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(picker.input.text(), "invoice");
        assert_eq!(picker.input.suggestion_suffix(), None);
    }

    #[test]
    fn percent_opens_scope_suggestions() {
        let mut picker = build_picker();
        picker.input.set_text("%");
        picker.refresh_filter();

        assert!(picker.input.is_suggestions_active());
        assert_eq!(
            picker
                .input
                .suggestions()
                .iter()
                .map(|suggestion| suggestion.value_to_store.as_str())
                .collect::<Vec<_>>(),
            vec!["%table ", "%profile ", "%dependency "]
        );
    }

    #[test]
    fn autocomplete_selected_accepts_scope_suggestion() {
        let mut picker = build_picker();
        picker.input.set_text("%p");
        picker.refresh_filter();

        let outcome = picker.autocomplete_selected();

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(picker.input.text(), "%profile ");
        assert!(!picker.input.is_suggestions_active());
    }

    #[test]
    fn scoped_clause_exposes_trailing_suffix_for_active_scope() {
        let mut picker = build_picker();
        picker.input.set_text("%profile fin");
        picker.refresh_filter();

        assert_eq!(picker.input.suggestion_suffix(), Some("ance"));
    }

    #[test]
    fn scoped_clause_with_trailing_space_exposes_value_suffix() {
        let mut picker = build_picker();
        picker.input.set_text("%profile ");
        picker.refresh_filter();

        assert_eq!(picker.input.suggestion_suffix(), Some("finance"));

        let outcome = picker.autocomplete_selected();

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(picker.input.text(), "%profile finance");
    }

    #[test]
    fn refresh_filter_experimentally_normalizes_picker_query_spaces() {
        let mut picker = build_picker();
        picker.input.set_text("  invoice   %profile   finance  ");
        picker.refresh_filter();

        assert_eq!(picker.input.text(), "invoice %profile finance ");
    }

    #[test]
    fn experimental_space_normalization_keeps_cursor_in_logical_position() {
        let mut picker = build_picker();
        picker.input.set_text("invoice  %profile");
        picker.input.set_cursor_position(9);
        picker.refresh_filter();

        assert_eq!(picker.input.text(), "invoice %profile");
        assert_eq!(picker.input.cursor_position(), 8);
    }

    #[test]
    fn autocomplete_after_extra_spaces_uses_normalized_query() {
        let mut picker = build_picker();
        picker.input.set_text("%profile  ");
        picker.refresh_filter();

        let outcome = picker.autocomplete_selected();

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(picker.input.text(), "%profile finance");
    }

    #[test]
    fn scoped_clause_can_limit_values() {
        let entries = vec![
            PickerEntry::new("amount", vec![]).with_fields(vec![
                PickerField::new("table", "invoice"),
                PickerField::new("column", "amount"),
            ]),
            PickerEntry::new("bonus", vec![]).with_fields(vec![
                PickerField::new("table", "department"),
                PickerField::new("column", "bonus"),
            ]),
        ];
        let mut picker = PickerData::<()>::with_entries(entries);
        picker.set_scopes(vec![
            PickerScope::new("table", "Table", vec!["table".to_string()])
                .with_completion_key("table")
                .with_value_token_limit(1),
            PickerScope::new("column", "Column", vec!["column".to_string()])
                .with_completion_key("column")
                .with_value_token_limit(1),
        ]);
        picker.input.set_text("%table department bo".to_string());
        picker.refresh_filter();

        let labels = picker
            .visible_entries()
            .map(|(_, entry)| entry.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["bonus"]);

        picker.input.set_text("%table department ".to_string());
        picker.refresh_filter();

        assert_eq!(picker.input.suggestion_suffix(), Some("bonus"));
    }

    #[test]
    fn suppressed_scope_value_hides_inline_suffix() {
        let entries = vec![
            PickerEntry::new("customer:1", vec![])
                .with_fields(vec![PickerField::new("target", "customer_id")]),
        ];
        let mut picker = PickerData::<()>::with_entries(entries);
        picker.set_scopes(vec![
            PickerScope::new("target", "Target", vec!["target".to_string()])
                .with_completion_key("target")
                .with_suppressed_value_completion(),
        ]);

        // Typing a value for the suppressed scope must not surface a suffix.
        picker.input.set_text("%target cust".to_string());
        picker.refresh_filter();
        assert_eq!(picker.input.suggestion_suffix(), None);

        // Trailing space after the value is still suppressed.
        picker.input.set_text("%target customer_id ".to_string());
        picker.refresh_filter();
        assert_eq!(picker.input.suggestion_suffix(), None);

        // The bare scope token (no value yet) still completes.
        picker.input.set_text("%target ".to_string());
        picker.refresh_filter();
        assert_eq!(picker.input.suggestion_suffix(), Some("customer_id"));
    }

    #[test]
    fn fuzzy_query_prefers_exact_label_over_secondary_field_match() {
        let entries = vec![
            PickerEntry::new("finance", vec![])
                .with_fields(vec![PickerField::new("profile", "ops")]),
            PickerEntry::new("invoice", vec![])
                .with_fields(vec![PickerField::new("profile", "finance")]),
        ];

        let mut picker = PickerData::<()>::with_entries(entries);
        picker.input.set_text("finance");
        picker.refresh_filter();

        let labels = picker
            .visible_entries()
            .map(|(_, entry)| entry.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["finance", "invoice"]);
    }

    #[test]
    fn fuzzy_query_matches_abbreviated_label() {
        let mut picker = build_picker();
        picker.input.set_text("invc");
        picker.refresh_filter();

        assert_eq!(
            picker.selected_entry().map(|entry| entry.label.as_str()),
            Some("invoice")
        );
    }

    #[test]
    fn implicit_scope_filters_when_query_has_no_explicit_scope() {
        let mut picker = build_picker();
        picker.set_implicit_scope("profile", "finance");

        let mut labels: Vec<_> = picker
            .visible_entries()
            .map(|(_, entry)| entry.label.as_str())
            .collect();
        labels.sort(); // Make comparison order-independent

        assert_eq!(labels, vec!["customer", "invoice"]);
    }

    #[test]
    fn explicit_scope_overrides_implicit_scope_of_same_token() {
        let mut picker = build_picker();
        picker.set_implicit_scope("profile", "finance");
        picker.input.set_text("%profile ops");
        picker.refresh_filter();

        let labels = picker
            .visible_entries()
            .map(|(_, entry)| entry.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["shipment"]);
    }

    #[test]
    fn bare_scoped_clause_filters_to_entries_with_matching_scope_fields() {
        let mut picker = build_picker();
        picker.input.set_text("%dependency");
        picker.refresh_filter();

        let labels = picker
            .visible_entries()
            .map(|(_, entry)| entry.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["invoice", "shipment"]);
    }
}
