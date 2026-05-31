use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use tui_pages::canvas;
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select,
    Quit,
}

struct State {
    body: canvas::TextAreaState<canvas::TextAreaProvider>,
    title: canvas::TextInputState<canvas::TextInputProvider>,
    message: String,
}

impl Default for State {
    fn default() -> Self {
        let mut body = canvas::TextAreaState::from_text("Write multiple lines here.\nTab inserts spaces.");
        body.use_wrap();

        let mut title = canvas::TextInputState::from_text("Draft");
        title.set_suggestion_suffix(" title");

        Self {
            body,
            title,
            message: "Textarea and TextInput are dispatched through tui-pages canvas helpers."
                .to_string(),
        }
    }
}

struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        _state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::Select => match ctx.focus {
                Some(FocusTarget::Button(0)) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

fn page_spec(_view: &View, _state: &State, focus: Option<&FocusTarget>) -> PageSpec {
    let spec = PageSpec::new().focus(
        PageFocusBuilder::new()
            .canvas_field(0)
            .canvas_field(1)
            .button(0),
    );
    if matches!(focus, Some(FocusTarget::Button(0))) {
        spec
    } else {
        spec.canvas_mode(canvas::AppMode::Edit)
    }
}

fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Editor)
        .page_fn(page_spec)
        .handler(Handler)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build()
}

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let _input = canvas::CrosstermInputSession::install()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = build();
    let mut state = State::default();
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| {
            let rows = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(frame.area());

            let title_focus = matches!(tui.focus.current(), Some(FocusTarget::CanvasField(0)));
            let title_block = block("Title", title_focus);
            let title_area = title_block.inner(rows[0]);
            frame.render_widget(title_block, rows[0]);
            frame.render_stateful_widget(
                canvas::TextInput::default().block(Block::default()),
                title_area,
                &mut state.title,
            );

            let body_focus = matches!(tui.focus.current(), Some(FocusTarget::CanvasField(1)));
            let body_block = block("Body", body_focus);
            let body_area = body_block.inner(rows[1]);
            frame.render_widget(body_block, rows[1]);
            frame.render_stateful_widget(
                canvas::TextArea::default().block(Block::default()),
                body_area,
                &mut state.body,
            );

            frame.render_widget(
                Paragraph::new(state.message.as_str())
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL).title(" status ")),
                rows[2],
            );
            frame.render_widget(
                Paragraph::new("Quit").block(Block::default().borders(Borders::ALL)),
                rows[3],
            );

            match tui.focus.current() {
                Some(FocusTarget::CanvasField(0)) => {
                    frame.set_cursor_position(state.title.cursor(title_area, None));
                }
                Some(FocusTarget::CanvasField(1)) => {
                    frame.set_cursor_position(state.body.cursor(body_area, None));
                }
                _ => {}
            }
        })?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        let handled = match tui.focus.current() {
            Some(FocusTarget::CanvasField(0)) => handle_text_widget(
                canvas::dispatch_text_input_key(&mut state.title, key),
                &mut tui,
                &state,
            ),
            Some(FocusTarget::CanvasField(1)) => handle_text_widget(
                canvas::dispatch_text_area_key(&mut state.body, key),
                &mut tui,
                &state,
            ),
            _ => false,
        };

        if !handled && tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}

fn handle_text_widget(
    outcome: canvas::CanvasTextWidgetOutcome,
    tui: &mut TuiApp<View, Action, State, Handler>,
    state: &State,
) -> bool {
    match outcome {
        canvas::CanvasTextWidgetOutcome::Handled => true,
        canvas::CanvasTextWidgetOutcome::Submitted => true,
        canvas::CanvasTextWidgetOutcome::Focus(intent) => {
            tui.apply_effect(TuiEffect::Focus(intent), state);
            true
        }
        canvas::CanvasTextWidgetOutcome::NotHandled => false,
    }
}

fn block(title: &'static str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(style)
}
