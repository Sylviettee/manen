use mlua::prelude::*;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crate::{format::{lua_to_string, TableFormat}, highlight::LuaHighlighter};

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

        let editor = Reedline::create()
            .with_highlighter(Box::new(LuaHighlighter::new()));

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
                    if let Err(e) = self.eval(&line) {
                        eprintln!("{e}")
                    }
                }
                //  TODO; this should cancel the current Lua execution if possible
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                    println!("aborted");
                    break
                },
                _ => {}
            }
        }
    }

    fn eval(&mut self, line: &str) -> LuaResult<()> {
        let value: LuaValue = self.lua.load(line).eval()?;

        let stringify = match value {
            LuaValue::Table(tbl) => self.table_format.format(&self.lua, &tbl)?,
            value => lua_to_string(&value)?,
        };

        // TODO; colorize
        println!("{stringify}");

        Ok(())
    }
}
