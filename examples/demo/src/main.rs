mod config;
mod pages;

use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::{
    modes, ActionContext, ActionOutcome, FocusIntent, FocusTarget, PageFocusBuilder, PageSpec,
    PaneSplit, TuiActionHandler, TuiEffect, TuiPages,
};

const OPTION_SECTION: usize = 0;
const OPTIONS: [&str; 4] = ["Fast path", "Detailed view", "Keep pane", "Show message"];
const BUTTONS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppView {
    Home,
    Options,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppAction {
    FocusNext,
    FocusPrev,
    MoveUp,
    MoveDown,
    Select,
    Home,
    Options,
    Details,
    NextBuffer,
    PreviousBuffer,
    SplitPane,
    NextPane,
    PreviousPane,
    ClosePane,
    Quit,
}

#[derive(Debug)]
struct AppState {
    selected_option: usize,
    message: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_option: 0,
            message: "Use 1/2/3 to open pages, [/] to switch buffers, v/p/x for panes.".into(),
        }
    }
}

struct Handler;

type DemoTui = TuiPages<
    AppView,
    AppAction,
    AppState,
    fn(&AppView, &AppState, Option<&FocusTarget>) -> PageSpec,
    Handler,
>;

fn page_spec(_view: &AppView, _state: &AppState, _focus: Option<&FocusTarget>) -> PageSpec {
    let mut focus = PageFocusBuilder::new().section(OPTION_SECTION);
    for index in 0..BUTTONS {
        focus = focus.button(index);
    }

    PageSpec::new()
        .focus_targets(focus.build())
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

impl TuiActionHandler<AppView, AppAction, AppState> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: AppAction,
        ctx: ActionContext<AppView>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<AppView>, Self::Error> {
        let outcome = match action {
            AppAction::FocusNext => move_top_level(ctx.focus, FocusIntent::Next),
            AppAction::FocusPrev => move_top_level(ctx.focus, FocusIntent::Prev),
            AppAction::MoveUp => move_inside_section(ctx.focus, FocusIntent::Prev),
            AppAction::MoveDown => move_inside_section(ctx.focus, FocusIntent::Next),
            AppAction::Select => select_focused(ctx, state),
            AppAction::Home => ActionOutcome::effect(TuiEffect::Navigate(AppView::Home)),
            AppAction::Options => ActionOutcome::effect(TuiEffect::Navigate(AppView::Options)),
            AppAction::Details => ActionOutcome::effect(TuiEffect::Navigate(AppView::Details)),
            AppAction::NextBuffer => ActionOutcome::effect(TuiEffect::NextBuffer),
            AppAction::PreviousBuffer => ActionOutcome::effect(TuiEffect::PreviousBuffer),
            AppAction::SplitPane => ActionOutcome::effect(TuiEffect::SplitPane(PaneSplit::Vertical)),
            AppAction::NextPane => ActionOutcome::effect(TuiEffect::NextPane),
            AppAction::PreviousPane => ActionOutcome::effect(TuiEffect::PreviousPane),
            AppAction::ClosePane => ActionOutcome::effect(TuiEffect::ClosePane),
            AppAction::Quit => ActionOutcome::effect(TuiEffect::Quit),
        };

        Ok(outcome)
    }
}

fn move_top_level(focus: Option<FocusTarget>, direction: FocusIntent) -> ActionOutcome<AppView> {
    if matches!(focus, Some(FocusTarget::SectionItem { .. })) {
        return ActionOutcome::effects([
            TuiEffect::Focus(FocusIntent::LeaveSection),
            TuiEffect::Focus(direction),
        ]);
    }

    ActionOutcome::effect(TuiEffect::Focus(direction))
}

fn move_inside_section(focus: Option<FocusTarget>, direction: FocusIntent) -> ActionOutcome<AppView> {
    if matches!(focus, Some(FocusTarget::SectionItem { .. })) {
        return ActionOutcome::effect(TuiEffect::Focus(direction));
    }

    ActionOutcome::none()
}

fn select_focused(ctx: ActionContext<AppView>, state: &mut AppState) -> ActionOutcome<AppView> {
    match ctx.focus {
        Some(FocusTarget::Section(OPTION_SECTION)) => {
            ActionOutcome::effect(TuiEffect::Focus(FocusIntent::EnterSection {
                item_count: OPTIONS.len(),
            }))
        }
        Some(FocusTarget::SectionItem {
            section: OPTION_SECTION,
            item,
        }) => {
            state.selected_option = item;
            state.message = format!("Selected option: {}", OPTIONS[item]);
            ActionOutcome::effect(TuiEffect::RefreshPage)
        }
        Some(FocusTarget::Button(0)) => ActionOutcome::effect(TuiEffect::Navigate(AppView::Home)),
        Some(FocusTarget::Button(1)) => {
            ActionOutcome::effect(TuiEffect::Navigate(AppView::Options))
        }
        Some(FocusTarget::Button(2)) => {
            ActionOutcome::effect(TuiEffect::Navigate(AppView::Details))
        }
        _ => {
            state.message = format!("Nothing to select on {:?}", ctx.current_view);
            ActionOutcome::effect(TuiEffect::RefreshPage)
        }
    }
}

fn build_tui(config: &config::Keybindings) -> DemoTui {
    let mut builder = TuiPages::builder(AppView::Home)
        .pages(page_spec as fn(&AppView, &AppState, Option<&FocusTarget>) -> PageSpec)
        .handler(Handler);

    builder = bind_all(builder, modes::GENERAL, &config.focus_next, AppAction::FocusNext);
    builder = bind_all(builder, modes::GENERAL, &config.focus_prev, AppAction::FocusPrev);
    builder = bind_all(builder, modes::GENERAL, &config.move_up, AppAction::MoveUp);
    builder = bind_all(builder, modes::GENERAL, &config.move_down, AppAction::MoveDown);
    builder = bind_all(builder, modes::GENERAL, &config.select, AppAction::Select);
    builder = bind_all(builder, modes::GENERAL, &config.home, AppAction::Home);
    builder = bind_all(builder, modes::GENERAL, &config.options, AppAction::Options);
    builder = bind_all(builder, modes::GENERAL, &config.details, AppAction::Details);
    builder = bind_all(builder, modes::GENERAL, &config.previous_buffer, AppAction::PreviousBuffer);
    builder = bind_all(builder, modes::GENERAL, &config.next_buffer, AppAction::NextBuffer);
    builder = bind_all(builder, modes::GENERAL, &config.split_pane, AppAction::SplitPane);
    builder = bind_all(builder, modes::GENERAL, &config.next_pane, AppAction::NextPane);
    builder = bind_all(builder, modes::GENERAL, &config.previous_pane, AppAction::PreviousPane);
    builder = bind_all(builder, modes::GENERAL, &config.close_pane, AppAction::ClosePane);
    builder = bind_all(builder, modes::GLOBAL, &config.quit, AppAction::Quit);

    builder.build()
}

fn bind_all<Pages, Handler>(
    mut builder: tui_pages::TuiPagesBuilder<AppView, AppAction, AppState, (), (), Pages, Handler>,
    mode: tui_pages::ModeId,
    bindings: &[String],
    action: AppAction,
) -> tui_pages::TuiPagesBuilder<AppView, AppAction, AppState, (), (), Pages, Handler> {
    for binding in bindings {
        builder = builder.bind(mode.clone(), binding, action);
    }
    builder
}

fn main() -> Result<()> {
    let config = config::Config::load();

    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(std::io::stderr());
    let mut terminal = Terminal::new(backend)?;
    let mut state = AppState::default();
    let mut tui = build_tui(&config.keybindings);
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| pages::render(frame, &tui, &state))?;

        match crossterm::event::read()? {
            crossterm::event::Event::Key(key) => {
                if tui.handle_key(key, &mut state)?.quit_requested {
                    break;
                }
            }
            crossterm::event::Event::Mouse(_) => tui.refresh_page(&state),
            crossterm::event::Event::Resize(_, _)
            | crossterm::event::Event::FocusGained
            | crossterm::event::Event::FocusLost
            | crossterm::event::Event::Paste(_) => {}
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), LeaveAlternateScreen)?;
    Ok(())
}
