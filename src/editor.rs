use mlua::prelude::*;
use reedline::{
    DefaultPrompt, DefaultPromptSegment, EditCommand, Emacs, KeyCode, KeyModifiers, Reedline,
    ReedlineEvent, Signal, default_emacs_keybindings,
};

use crate::{
    format::TableFormat,
    highlight::LuaHighlighter,
    inspect::{display_basic, display_table},
    validator::LuaValidator,
};

pub struct Editor {
    prompt: DefaultPrompt,
    editor: Reedline,
    lua: Lua,

    table_format: TableFormat,
}

impl Editor {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let version: String = lua.globals().get("_VERSION")?;

        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic(version),
            DefaultPromptSegment::Empty,
        );

        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Enter,
            ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
        );

        let editor = Reedline::create()
            .with_highlighter(Box::new(LuaHighlighter::new()))
            .with_validator(Box::new(LuaValidator::new()))
            .with_hinter(Box::new(LuaValidator::new()))
            .with_edit_mode(Box::new(Emacs::new(keybindings)));

        Ok(Self {
            prompt,
            editor,
            lua,

            table_format: TableFormat::ComfyTable(true),
        })
    }

    pub fn run(mut self) {
        loop {
            let signal = self.editor.read_line(&self.prompt);

            match signal {
                Ok(Signal::Success(line)) => {
                    if line.starts_with(".") {
                        if let Err(e) = self.eval_special(&line) {
                            eprintln!("{e}")
                        }

                        continue;
                    }

                    if let Err(e) = self.eval(&line) {
                        eprintln!("{e}")
                    }
                }
                //  TODO; this should cancel the current Lua execution if possible
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => break,
                _ => {}
            }
        }
    }

    // .help
    // .format <format> [true/false]
    fn eval_special(&mut self, line: &str) -> LuaResult<()> {
        let mut split = line.strip_prefix(".").unwrap().split_whitespace();

        let cmd = split.next();

        match cmd {
            Some("help") => {
                println!(".help\tPrint this message");
                println!(
                    ".format <inspect|address|comfytable> [true|false]\tConfigure table printing, boolean configures nesting"
                );
            }
            Some("format") => match split.next() {
                Some("inspect") => {
                    self.table_format = TableFormat::Inspect;
                }
                Some("address") => {
                    self.table_format = TableFormat::Address;
                }
                Some("comfytable") => {
                    let nested = split
                        .next()
                        .unwrap_or("")
                        .parse::<bool>()
                        .unwrap_or_default();

                    self.table_format = TableFormat::ComfyTable(nested);
                }
                _ => println!("unknown subcommand"),
            },
            _ => println!("unknown command"),
        }

        Ok(())
    }

    fn eval(&mut self, line: &str) -> LuaResult<()> {
        let value: LuaValue = self.lua.load(line).set_name("=stdin").eval()?;

        let stringify = match value {
            LuaValue::Table(tbl) => display_table(&tbl, true).unwrap(), //  self.table_format.format(&self.lua, &tbl, true)?,
            value => display_basic(&value, true),
        };

        // TODO; colorize
        println!("{stringify}");

        Ok(())
    }
}
