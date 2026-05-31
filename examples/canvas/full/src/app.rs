use tui_pages::canvas;
use tui_pages::prelude::*;

use crate::{form, notes, search};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Form,
    Notes,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Canvas(canvas::CanvasAction),
    FocusNext,
    FocusPrev,
    GotoForm,
    GotoNotes,
    GotoSearch,
    Select,
    Quit,
}

impl From<canvas::CanvasAction> for Action {
    fn from(action: canvas::CanvasAction) -> Self {
        Self::Canvas(action)
    }
}

#[derive(Debug)]
pub struct InvoiceProvider {
    values: Vec<String>,
}

impl Default for InvoiceProvider {
    fn default() -> Self {
        Self {
            values: vec![
                "consulting".to_string(),
                "2".to_string(),
                "150".to_string(),
                String::new(),
            ],
        }
    }
}

impl canvas::DataProvider for InvoiceProvider {
    fn field_count(&self) -> usize {
        self.values.len()
    }

    fn field_name(&self, index: usize) -> &str {
        match index {
            0 => "Item",
            1 => "Qty",
            2 => "Price",
            3 => "Total",
            _ => "",
        }
    }

    fn field_value(&self, index: usize) -> &str {
        &self.values[index]
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.values[index] = value;
    }

    fn supports_suggestions(&self, field_index: usize) -> bool {
        field_index == 0
    }

    fn suggestion_trigger(&self, field_index: usize) -> canvas::SuggestionTrigger {
        if field_index == 0 {
            canvas::SuggestionTrigger::WhenFieldStarts
        } else {
            canvas::SuggestionTrigger::None
        }
    }

    fn fetch_suggestions_sync(
        &self,
        _field_index: usize,
        query: &str,
    ) -> Vec<canvas::SuggestionItem> {
        ["consulting", "design", "implementation", "support"]
            .into_iter()
            .filter(|item| item.starts_with(query))
            .map(|item| canvas::SuggestionItem::new(item, item))
            .collect()
    }

    fn validation_config(&self, field_index: usize) -> Option<canvas::ValidationConfig> {
        match field_index {
            1 | 2 => Some(
                canvas::ValidationConfigBuilder::new()
                    .with_pattern_filters(canvas::PatternFilters::new().add_filter(
                        canvas::PositionFilter::new(
                            canvas::PositionRange::From(0),
                            canvas::CharacterFilter::Numeric,
                        ),
                    ))
                    .build(),
            ),
            _ => None,
        }
    }

    fn is_computed_field(&self, field_index: usize) -> bool {
        field_index == 3
    }
}

pub struct TotalComputer;

impl canvas::ComputedProvider for TotalComputer {
    fn compute_field(&mut self, context: canvas::ComputedContext) -> String {
        let qty = context.field_values[1].parse::<i64>().unwrap_or_default();
        let price = context.field_values[2].parse::<i64>().unwrap_or_default();
        (qty * price).to_string()
    }

    fn handles_field(&self, field_index: usize) -> bool {
        field_index == 3
    }

    fn field_dependencies(&self, _field_index: usize) -> Vec<usize> {
        vec![1, 2]
    }
}

pub struct AppState {
    pub form: canvas::FormEditor<InvoiceProvider>,
    pub notes: canvas::TextAreaState<canvas::TextAreaProvider>,
    pub search: canvas::TextInputState<canvas::TextInputProvider>,
    pub message: String,
}

impl Default for AppState {
    fn default() -> Self {
        let mut form = canvas::FormEditor::new(InvoiceProvider::default());
        let mut computer = TotalComputer;
        form.register_computed_provider(&computer);
        form.recompute_all_fields(&mut computer);

        let mut notes = canvas::TextAreaState::from_text("Each page demonstrates a different canvas surface.");
        notes.use_wrap();

        let mut search = canvas::TextInputState::from_text("con");
        search.set_suggestion_suffix("sulting");

        Self {
            form,
            notes,
            search,
            message: "g f / g n / g s changes pages. Ctrl+C quits.".to_string(),
        }
    }
}

pub type App = TuiApp<View, Action, AppState, Handler>;

pub struct Handler;

impl TuiActionHandler<View, Action, AppState> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        if let Some(outcome) = global_action(&action) {
            return Ok(outcome);
        }

        Ok(match ctx.current_view {
            View::Form => form::page::handle(action, &ctx, state),
            View::Notes => notes::page::handle(action, &ctx, state),
            View::Search => search::page::handle(action, &ctx, state),
        })
    }
}

fn global_action(action: &Action) -> Option<ActionOutcome<View>> {
    Some(match action {
        Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
        Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
        Action::GotoForm => ActionOutcome::effect(TuiEffect::Navigate(View::Form)),
        Action::GotoNotes => ActionOutcome::effect(TuiEffect::Navigate(View::Notes)),
        Action::GotoSearch => ActionOutcome::effect(TuiEffect::Navigate(View::Search)),
        Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        Action::Canvas(_) | Action::Select => return None,
    })
}

fn page_spec(view: &View, state: &AppState, focus: Option<&FocusTarget>) -> PageSpec {
    match view {
        View::Form => form::page::page_spec(state, focus),
        View::Notes => notes::page::page_spec(state, focus),
        View::Search => search::page::page_spec(state, focus),
    }
}

pub fn build() -> App {
    TuiPages::builder(View::Form)
        .page_fn(page_spec)
        .handler(Handler)
        .canvas_defaults()
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GLOBAL, "g f", Action::GotoForm)
        .bind(modes::GLOBAL, "g n", Action::GotoNotes)
        .bind(modes::GLOBAL, "g s", Action::GotoSearch)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build()
}
