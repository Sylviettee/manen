use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use editor::Editor;
use highlight::LuaHighlighter;
use reedline::Highlighter;

mod editor;
mod format;
mod highlight;
mod inspect;
mod validator;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Repl,
    Highlight { path: Option<PathBuf> },
}

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        None | Some(Command::Repl) => Editor::new()?.run(),
        Some(Command::Highlight { path }) => {
            let file = if let Some(path) = path {
                fs::read_to_string(path)?
            } else {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;

                buffer
            };

            let highlighter = LuaHighlighter::new();
            let text = highlighter.highlight(&file, 0);

            println!("{}", text.render_simple());
        }
    }

    Ok(())
}
