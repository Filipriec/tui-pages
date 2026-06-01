// The wiring layer. Shared types, the form's backing data, the key bindings, and
// the routers that fan out to the page modules (form/, editor/, help/). No page
// rendering or page logic lives here — each page owns that in its own folder.
//
// The two canvas widgets are registered once on the builder:
//   .canvas_form_editor(0)      → the Form page's FormEditor
//   .canvas_textarea_widget(0)  → the Editor page's TextArea
// Both are addressed by focus index 0, so `CanvasWidgetState` disambiguates them
// by the current view: only the widget on the visible page hands back a host.
//
// `Overlay::CommandBar` is the `:` command palette — plain app state we drive
// ourselves in main.rs and only hand to `submit_command` on Enter.

use tui_pages::canvas;
use tui_pages::prelude::*;

use crate::{editor, form, help};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overlay {
    CommandBar,
}

/// Application-owned "which dialog is this" payload, carried by the built-in
/// dialog modal. Opaque to the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    PostLogin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Form,
    Editor,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    FocusNext,
    FocusPrev,
    GotoForm,
    GotoEditor,
    GotoHelp,
    OpenPalette,
    Select,
    Quit,
}

/// The Form page's backing data: two fields the editor reads and writes.
#[derive(Debug)]
pub struct Contact {
    pub values: Vec<String>,
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            values: vec!["Ada".to_string(), "ada@example.test".to_string()],
        }
    }
}

impl canvas::DataProvider for Contact {
    fn field_count(&self) -> usize {
        self.values.len()
    }

    fn field_name(&self, index: usize) -> &str {
        match index {
            0 => "Name",
            1 => "Email",
            _ => "",
        }
    }

    fn field_value(&self, index: usize) -> &str {
        &self.values[index]
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.values[index] = value;
    }
}

/// Application state. `view` mirrors the runtime's current view so the
/// `CanvasWidgetState` impl can route a canvas key to the widget that is
/// actually on screen. `entered` tracks whether the textarea has been entered
/// for inner navigation. `palette_*` back the `:` command bar.
pub struct AppState {
    pub view: View,
    pub form: canvas::FormEditor<Contact>,
    pub body: canvas::TextAreaState<canvas::TextAreaProvider>,
    pub entered: bool,
    pub message: String,
    pub palette_open: bool,
    pub palette_input: String,
}

impl Default for AppState {
    fn default() -> Self {
        let mut body = canvas::TextAreaState::from_text(
            "Enter selects this textarea.\nThen i edits, Esc leaves edit mode.\nj/k move between lines once you are inside.\nWhile it is just a stop, j/k jump straight to the buttons.",
        );
        body.use_wrap();

        Self {
            view: View::Form,
            form: canvas::FormEditor::new(Contact::default()),
            body,
            entered: false,
            message: "g f / g e / g ? switches pages. `:` opens the palette. Ctrl+C quits."
                .to_string(),
            palette_open: false,
            palette_input: String::new(),
        }
    }
}

impl canvas::CanvasWidgetState for AppState {
    fn canvas_form_editor(&mut self, id: usize) -> Option<&mut dyn canvas::CanvasFormEditorHost> {
        match (self.view, id) {
            (View::Form, 0) => Some(&mut self.form),
            _ => None,
        }
    }

    fn canvas_textarea(
        &mut self,
        focus_index: usize,
    ) -> Option<&mut dyn canvas::CanvasTextAreaHost> {
        match (self.view, focus_index) {
            (View::Editor, 0) => Some(&mut self.body),
            _ => None,
        }
    }

    fn canvas_textarea_entered(&mut self, focus_index: usize) -> Option<&mut bool> {
        match (self.view, focus_index) {
            (View::Editor, 0) => Some(&mut self.entered),
            _ => None,
        }
    }
}

// The runtime's modal payload `M` is `DialogData<Purpose>`, so the focus manager
// owns the login dialog's content and tracks its active button.
pub type App = TuiApp<View, Action, AppState, Handler, Overlay, DialogData<Purpose>>;

pub struct Handler;

impl TuiActionHandler<View, Action, AppState, Overlay, DialogData<Purpose>> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View, Overlay>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<View, Overlay, DialogData<Purpose>>, Self::Error> {
        // Actions that mean the same thing everywhere are handled once; the rest
        // are routed to whichever page we're on.
        if let Some(outcome) = global_action(&action, state) {
            return Ok(outcome);
        }

        Ok(match ctx.current_view {
            View::Form => form::handle(action, &ctx, state),
            View::Editor => editor::handle(action, &ctx, state),
            View::Help => help::handle(action, &ctx, state),
        })
    }
}

fn global_action(
    action: &Action,
    state: &mut AppState,
) -> Option<ActionOutcome<View, Overlay, DialogData<Purpose>>> {
    Some(match action {
        Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
        Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
        Action::GotoForm => ActionOutcome::effect(TuiEffect::Navigate(View::Form)),
        Action::GotoEditor => ActionOutcome::effect(TuiEffect::Navigate(View::Editor)),
        Action::GotoHelp => ActionOutcome::effect(TuiEffect::Navigate(View::Help)),
        Action::OpenPalette => {
            state.palette_open = true;
            state.palette_input.clear();
            ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Open(FocusTarget::Overlay(
                Overlay::CommandBar,
            ))))
        }
        Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        // Select depends on where you are — let the page decide.
        Action::Select => return None,
    })
}

fn page_spec(
    view: &View,
    state: &AppState,
    _focus: Option<&FocusTarget<Overlay>>,
) -> PageSpec<Overlay> {
    match view {
        View::Form => form::page_spec(state),
        View::Editor => editor::page_spec(state),
        View::Help => help::page_spec(state),
    }
}

pub fn build() -> App {
    TuiPages::builder(View::Form)
        .page_fn(page_spec)
        .handler(Handler)
        // Attach the two canvas widgets. Each handles its own canvas keys
        // (i/a for modes, j/k/h/l movement, enter/edit/exit) internally, so
        // those never reach our Action type.
        .canvas_form_editor(0)
        .canvas_textarea_widget(0)
        // Top-level / button navigation (general mode). j/k/h/l flow continues
        // straight off the canvas boundary onto the buttons.
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "backtab", Action::FocusPrev)
        .bind(modes::GENERAL, "j", Action::FocusNext)
        .bind(modes::GENERAL, "k", Action::FocusPrev)
        .bind(modes::GENERAL, "l", Action::FocusNext)
        .bind(modes::GENERAL, "h", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GENERAL, ":", Action::OpenPalette)
        .bind(modes::GLOBAL, "g f", Action::GotoForm)
        .bind(modes::GLOBAL, "g e", Action::GotoEditor)
        .bind(modes::GLOBAL, "g ?", Action::GotoHelp)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        // Command palette entries (`:` then type, Enter runs).
        .command("Go to Form", ["f", "form"], Action::GotoForm)
        .command("Go to Editor", ["e", "editor"], Action::GotoEditor)
        .command("Go to Help", ["?", "help"], Action::GotoHelp)
        .command("Quit", ["q", "quit"], Action::Quit)
        .build()
}
