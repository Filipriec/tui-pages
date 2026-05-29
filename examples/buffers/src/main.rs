mod app;
mod ui;

use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let mut tui = app::build();
    let mut state = ();

    loop {
        // The renderer reads the buffer state the runtime maintains for us.
        terminal.draw(|frame| ui::render(frame, &tui.buffer))?;
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
