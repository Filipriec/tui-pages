// Everything in this file talks to `tui-pages`. The UI layer (ui.rs) does not.

use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Standard navigation actions provided by the keybinding presets.
    Nav(NavigationAction),
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
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
        match action {
            // Activate is per-page — let the page decide what "enter" means
            // for the currently focused target.
            Action::Nav(NavigationAction::Activate) => match (ctx.current_view, ctx.focus) {
                (View::Home, Some(FocusTarget::Button(0))) => {
                    Ok(ActionOutcome::effect(TuiEffect::Navigate(View::About)))
                }
                (View::About, Some(FocusTarget::Button(0))) => {
                    Ok(ActionOutcome::effect(TuiEffect::Navigate(View::Home)))
                }
                _ => Ok(ActionOutcome::none()),
            },
            Action::Nav(nav) => Ok(ActionOutcome::effect(nav.to_effect())),
        }
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
        // Vim preset covers the full surface: focus movement (tab/arrows/hjkl),
        // activate (enter), leave section (esc), and quit (ctrl+c).
        .vim_defaults()
        .build();
    app.refresh_page(&());
    app
}
