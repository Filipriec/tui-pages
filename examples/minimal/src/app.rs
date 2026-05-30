// Everything in this file talks to `tui-pages`. The UI layer (ui.rs) does not.

use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    About,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    FocusNext,
    FocusPrev,
    Select,
    Quit,
}

pub type App = TuiApp<View, Action, (), Handler>;

pub struct Handler;

impl TuiActionHandler<View, Action, ()> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        _state: &mut (),
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
            Action::Select => match (ctx.current_view, ctx.focus) {
                (View::Home, Some(FocusTarget::Button(0))) => {
                    ActionOutcome::effect(TuiEffect::Navigate(View::About))
                }
                (View::About, Some(FocusTarget::Button(0))) => {
                    ActionOutcome::effect(TuiEffect::Navigate(View::Home))
                }
                (_, Some(FocusTarget::Button(1))) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
        })
    }
}

fn page_spec(_view: &View, _state: &(), _focus: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus_targets(PageFocusBuilder::new().button(0).button(1).build())
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn build() -> App {
    let mut app = TuiPages::builder(View::Home)
        .page_fn(page_spec)
        .handler(Handler)
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build();
    app.refresh_page(&());
    app
}
