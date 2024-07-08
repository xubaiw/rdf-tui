mod app;
mod util;

use crate::app::App;
use clap::Parser;
use util::{restore_terminal, setup_terminal};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut terminal = setup_terminal()?;
    let mut app = App::new()?;

    if let Some(path) = args.path {
        app.load(path)?;
    }

    app.run(&mut terminal)?;

    restore_terminal();

    Ok(())
}

#[derive(Debug, Parser)]
pub struct Args {
    path: Option<String>,
}
