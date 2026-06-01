mod app;
mod ui;

use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    // One call sets up raw mode + the alternate screen and hands back a guard
    // that restores the terminal when it drops — including if the loop panics.
    let _guard = tui_pages::terminal::enter()?;
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

    Ok(())
}
