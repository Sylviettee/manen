use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};
use editor::Editor;
use highlight::LuaHighlighter;
use mlua::prelude::*;
use reedline::Highlighter;

use crate::{format::comfy_table, inspect::inspect};

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
    /// Enter an interactive REPL session
    Repl,
    /// Run a Lua file
    Run {
        /// Path to Lua file 
        path: PathBuf
    },
    /// Highlight a Lua file
    Highlight {
        /// Path to Lua file (default: stdin)
        path: Option<PathBuf>
    },
}

fn eval_lua(file: String, path: &Path) -> LuaResult<()> {
    let lua = Lua::new();
    let globals = lua.globals();

    globals.raw_set(
        "inspect",
        lua.create_function(|_, (value, colorize): (LuaValue, Option<bool>)| {
            println!("{}", inspect(&value, colorize.unwrap_or(true))?);
            Ok(())
        })?,
    )?;

    globals.raw_set(
        "comfytable",
        lua.create_function(|_, (table, recursive): (LuaTable, Option<bool>)| {
            println!("{}", comfy_table(&table, recursive.unwrap_or(true))?);

            Ok(())
        })?,
    )?;

    let res = lua
        .load(file)
        .set_name(format!("@{}", path.to_string_lossy()))
        .eval::<LuaMultiValue>();

    match res {
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
        Ok(values) => {
            for value in values {
                println!("{}", inspect(&value, true)?);
            }

            Ok(())
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match &cli.command {
        None | Some(Command::Repl) => Editor::new()?.run(),
        Some(Command::Run { path }) => {
            eval_lua(fs::read_to_string(path)?, path)?;
        }
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
