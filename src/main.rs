use std::{
    fs,
    io::{Read, stdin},
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};
use editor::Editor;
use emmylua_parser::{LuaParser, ParserConfig};
use mlua::prelude::*;
use reedline::Highlighter;

use format::comfy_table;
use inspect::inspect;
use parse::LuaHighlighter;

mod completion;
mod editor;
mod format;
mod hinter;
mod inspect;
mod parse;
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
        path: PathBuf,
    },
    /// Highlight a Lua file
    Highlight {
        /// Path to Lua file (default: stdin)
        path: Option<PathBuf>,
    },
    /// DEBUG: Parse a Lua file with emmylua_parser
    Parse { path: PathBuf },
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
                stdin().read_to_string(&mut buffer)?;

                buffer
            };

            let text = LuaHighlighter.highlight(&file, 0);

            println!("{}", text.render_simple());
        }
        Some(Command::Parse { path }) => {
            let code = fs::read_to_string(path)?;

            let tree = LuaParser::parse(&code, ParserConfig::default());

            parse::debug_tree(&tree);
        }
    }

    Ok(())
}
