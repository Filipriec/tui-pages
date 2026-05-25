use crate::focus::{FocusIntent, FocusTarget};
use crate::input::{InputHint, KeyChord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResolution<A, P> {
    PageLogic { page_id: P, action: A },
    CanvasLogic { action: A },
    GlobalLogic(A),
    Type(KeyChord),
    Wait(Vec<InputHint<A>>),
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageActionResult<AppAction, D = (), P = ()> {
    pub app_action: Option<AppAction>,
    pub focus_intent: Option<FocusIntent<D, P>>,
}

impl<AppAction, D, P> Default for PageActionResult<AppAction, D, P> {
    fn default() -> Self {
        Self {
            app_action: None,
            focus_intent: None,
        }
    }
}

impl<AppAction, D, P> PageActionResult<AppAction, D, P> {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn app(action: AppAction) -> Self {
        Self {
            app_action: Some(action),
            focus_intent: None,
        }
    }

    pub fn focus(intent: FocusIntent<D, P>) -> Self {
        Self {
            app_action: None,
            focus_intent: Some(intent),
        }
    }

    pub fn with(app_action: Option<AppAction>, focus_intent: Option<FocusIntent<D, P>>) -> Self {
        Self {
            app_action,
            focus_intent,
        }
    }
}

pub trait PageActionHandler<Action, AppAction, AppState, Error, D = (), P = ()> {
    fn handle_action(
        &mut self,
        app_state: &AppState,
        focus: &FocusTarget,
        action: &Action,
    ) -> Result<PageActionResult<AppAction, D, P>, Error>;

    fn handle_button(
        &mut self,
        _index: usize,
    ) -> Result<PageActionResult<AppAction, D, P>, Error> {
        Ok(PageActionResult::none())
    }
}
