use tui_pages::prelude::*;

use super::logic;
use crate::app::{Action, AppState, Overlay, View};

pub fn page_spec(_state: &AppState) -> PageSpec<Overlay> {
    // The item count rides along with the section, so the runtime can step
    // through the list on its own once it's entered.
    PageSpec::new()
        .focus(
            PageFocusBuilder::new()
                .section_with_items(logic::SECTION, logic::NOTES.len())
                .button(0),
        )
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn handle(
    action: Action,
    ctx: &ActionContext<View, Overlay>,
    state: &mut AppState,
) -> ActionOutcome<View, Overlay> {
    match action {
        Action::Select => match &ctx.focus {
            Some(FocusTarget::SectionItem {
                section: logic::SECTION,
                item,
            }) => {
                logic::select(state, *item);
                ActionOutcome::effect(TuiEffect::RefreshPage)
            }
            Some(FocusTarget::Button(0)) => ActionOutcome::effect(TuiEffect::Navigate(View::Home)),
            _ => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Activate)),
        },
        _ => ActionOutcome::none(),
    }
}
