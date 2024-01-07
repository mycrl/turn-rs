mod events;
mod state;
mod ui;
mod util;

use std::{
    io::{self, stdout},
    sync::Arc,
};

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use events::EventProxy;
use ratatui::prelude::*;
use state::create_state;
use turn_driver::controller::Controller;
use ui::Ui;
use util::Opts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let rpc = Arc::new(Controller::new(&opts.uri).await?);
    let (event_proxy, receiver) = EventProxy::new();
    let state = create_state(rpc, receiver).await;

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let ui = Ui::new(event_proxy, state);
    let mut should_quit = false;
    while !should_quit {
        should_quit = handle_events(&ui)?;
        terminal.draw(|frame| {
            ui.draw(frame);
        })?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events(ui: &Ui) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if key.code == KeyCode::Char('q') {
                    return Ok(true);
                } else {
                    ui.input(key.code);
                }
            }
        }
    }

    Ok(false)
}
