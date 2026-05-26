use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use tui_pages::{
    modes, ActionContext, ActionOutcome, FocusIntent, FocusTarget, PageFocusBuilder, PageSpec,
    TuiActionHandler, TuiEffect, TuiPages,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Home,
    About,
}

#[derive(Debug, Clone, Copy)]
enum Action {
    FocusNext,
    FocusPrev,
    Select,
    Quit,
}

struct Handler;

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

fn render(frame: &mut Frame, view: View, focus: Option<FocusTarget>) {
    let (body, primary) = match view {
        View::Home => ("Welcome to the Home page.", "Go to About"),
        View::About => ("This is the About page.", "Back to Home"),
    };

    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    render_tabs(frame, rows[0], view);

    frame.render_widget(
        Paragraph::new(body).alignment(Alignment::Center).block(
            Block::default()
                .title(format!(" {} ", view_name(view)))
                .borders(Borders::ALL),
        ),
        rows[1],
    );

    render_button(frame, rows[2], primary, &focus, 0);
    render_button(frame, rows[3], "Quit", &focus, 1);

    frame.render_widget(
        Paragraph::new("Tab / Shift+Tab to move focus  ·  Enter to select  ·  Ctrl+C to quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        rows[4],
    );
}

fn view_name(view: View) -> &'static str {
    match view {
        View::Home => "Home",
        View::About => "About",
    }
}

fn render_tabs(frame: &mut Frame, area: ratatui::layout::Rect, view: View) {
    let tab = |name: &str, active: bool| {
        if active {
            Span::styled(
                format!(" [{name}] "),
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!("  {name}  "), Style::default().fg(Color::DarkGray))
        }
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            tab("Home", matches!(view, View::Home)),
            Span::raw("  "),
            tab("About", matches!(view, View::About)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_button(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    label: &str,
    focus: &Option<FocusTarget>,
    index: usize,
) {
    let focused = matches!(focus, Some(FocusTarget::Button(i)) if *i == index);
    let style = if focused {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(style)
            .block(Block::default().borders(Borders::ALL).border_style(style)),
        area,
    );
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let mut tui = TuiPages::builder(View::Home)
        .pages(page_spec as fn(&View, &(), Option<&FocusTarget>) -> PageSpec)
        .handler(Handler)
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build();

    let mut state = ();
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| render(frame, *tui.current_view(), tui.focus.current()))?;
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            if tui.handle_key(key, &mut state)?.quit_requested {
                break;
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), LeaveAlternateScreen)?;
    Ok(())
}
