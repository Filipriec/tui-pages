//! An interactive demo of tui-pages owning keybinding load **and** save.
//!
//! Run with `cargo run` from this directory. The app:
//!   * reads its keybindings from `config.toml` (the unified `[keymap.*]`
//!     schema) at startup, resolving names through an [`ActionRegistry`];
//!   * runs a normal ratatui event loop where those bindings drive it —
//!     Tab/arrows move focus, Enter activates, the configured keys toggle the
//!     sidebar and quit;
//!   * rebinds the toggle key at runtime (`cycle_toggle_key`) and writes the
//!     live keybindings back to `config.toml` (`save_config`) via
//!     `export_keybindings_toml()` — so the next launch picks them up.
//!
//! All keybinding plumbing is the crate's; the app just supplies its action
//! names and renders.

mod ui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::keybindings::ActionRegistry;
use tui_pages::prelude::*;
use tui_pages::BindableActionInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Nav(NavigationAction),
    ToggleSidebar,
    SaveConfig,
    CycleToggleKey,
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
}

#[derive(Default)]
pub struct State {
    pub sidebar_open: bool,
    pub status: String,
    /// Set by the handler; performed by the event loop (which can touch the app
    /// and the filesystem, unlike the action handler).
    pub save_requested: bool,
    pub cycle_requested: bool,
}

struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        match action {
            Action::ToggleSidebar => {
                state.sidebar_open = !state.sidebar_open;
                state.status = format!(
                    "sidebar {}",
                    if state.sidebar_open { "opened" } else { "closed" }
                );
                Ok(ActionOutcome::none())
            }
            Action::SaveConfig => {
                state.save_requested = true;
                Ok(ActionOutcome::none())
            }
            Action::CycleToggleKey => {
                state.cycle_requested = true;
                Ok(ActionOutcome::none())
            }
            Action::Nav(nav) => Ok(ActionOutcome::effect(nav.to_effect())),
        }
    }
}

fn page_spec(_view: &View, _state: &State, _focus: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus_targets(PageFocusBuilder::new().button(0).button(1).build())
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

/// The app's vocabulary: built-in navigation actions + this app's own, each
/// tagged with the config name it answers to. Load and save both use it.
fn action_registry() -> ActionRegistry<Action> {
    let mut registry = ActionRegistry::navigation();
    registry.extend([
        info(Action::ToggleSidebar, "toggle_sidebar", "Show/hide the sidebar"),
        info(Action::SaveConfig, "save_config", "Write keybindings to config.toml"),
        info(Action::CycleToggleKey, "cycle_toggle_key", "Rebind the sidebar key"),
    ]);
    registry
}

fn info(action: Action, name: &'static str, description: &'static str) -> BindableActionInfo<Action> {
    BindableActionInfo {
        action,
        name,
        description,
        modes: &["global"],
    }
}

fn config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml")
}

type App = TuiApp<View, Action, State, Handler>;

fn build_app(config_toml: &str) -> Result<App> {
    let app = TuiPages::builder(View::Main)
        .page_fn(page_spec)
        .handler(Handler)
        // A built-in fallback so Ctrl+C always quits even with an empty config.
        .bind(modes::GLOBAL, "ctrl+c", Action::Nav(NavigationAction::Quit))
        .action_registry(action_registry())
        .keybindings_toml(config_toml)
        .context("config.toml has invalid keybindings")?
        .build();
    Ok(app)
}

/// The key currently bound to an action in the live keymap (for the footer).
fn current_key(app: &App, action: &Action) -> String {
    for map in app.input.registry.maps.values() {
        for (sequence, bound) in &map.bindings {
            if bound == action {
                return sequence
                    .iter()
                    .map(|chord| chord.display_string())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
        }
    }
    "(unbound)".to_string()
}

fn main() -> Result<()> {
    let path = config_path();
    let config_toml = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut app = build_app(&config_toml)?;
    let mut state = State {
        status: format!("loaded keybindings from {}", path.display()),
        ..State::default()
    };
    app.refresh_page(&state);

    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    loop {
        let toggle_key = current_key(&app, &Action::ToggleSidebar);
        let save_key = current_key(&app, &Action::SaveConfig);
        let cycle_key = current_key(&app, &Action::CycleToggleKey);
        let quit_key = current_key(&app, &Action::Nav(NavigationAction::Quit));
        terminal.draw(|frame| {
            ui::render(
                frame,
                &state,
                app.focus.current(),
                &ui::Keys {
                    toggle: toggle_key,
                    save: save_key,
                    cycle: cycle_key,
                    quit: quit_key,
                },
            )
        })?;

        let crossterm::event::Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        let output = app.handle_key(key, &mut state)?;

        if std::mem::take(&mut state.cycle_requested) {
            // Move the sidebar toggle between Ctrl+B and Ctrl+G at runtime. The
            // rebind reaches the live keymap immediately (the footer updates).
            let next = if current_key(&app, &Action::ToggleSidebar) == "Ctrl+b" {
                "ctrl+g"
            } else {
                "ctrl+b"
            };
            app.rebind_keymap("global", next, Action::ToggleSidebar)?;
            state.status = format!("rebound toggle_sidebar -> {next}");
        }

        if std::mem::take(&mut state.save_requested) {
            let exported = app.export_keybindings_toml()?;
            std::fs::write(&path, &exported)
                .with_context(|| format!("failed to write {}", path.display()))?;
            state.status = format!("saved keybindings to {}", path.display());
        }

        if output.quit_requested {
            break;
        }
    }

    Ok(())
}
