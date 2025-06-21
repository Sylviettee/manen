use std::collections::{HashMap, HashSet};

use comfy_table::{presets::UTF8_FULL, Table};
use mlua::prelude::*;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crate::highlight::LuaHighlighter;

const INSPECT_CODE: &str = include_str!("inspect.lua");

pub enum TableFormat {
    ComfyTable(bool),
    Inspect,
    Address,
}

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

        let inspect: LuaTable = lua.load(INSPECT_CODE).eval()?;
        lua.globals().set("_inspect", inspect)?;

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

            table_format: TableFormat::Inspect,
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

    fn addr_tbl(tbl: &LuaTable) -> String {
        format!("table@{:?}", tbl.to_pointer())
    }

    fn convert_string(string: &LuaString) -> String {
        let bytes = string
            .as_bytes()
            .iter()
            .flat_map(|b| std::ascii::escape_default(*b))
            .collect::<Vec<_>>();

        String::from_utf8_lossy(&bytes).to_string()
    }

    fn lua_to_string(value: &LuaValue) -> LuaResult<String> {
        match value {
            LuaValue::String(string) => Ok(Self::convert_string(string)),
            LuaValue::Table(tbl) => Ok(Self::addr_tbl(tbl)),
            value => value.to_string()
        }
    }

    fn pretty_table(tbl: &LuaTable, recursive: bool, visited: &mut HashMap<String, usize>) -> LuaResult<String> {
        let addr = Self::addr_tbl(tbl);

        if let Some(id) = visited.get(&addr) {
            return Ok(format!("<table {id}>"))
        }
        
        let id = visited.len();
        visited.insert(addr.clone(), id);

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec![format!("<table {id}>")]);

        for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
            let (key_str, value_str) = if let LuaValue::Table(sub) = value {
                if recursive {
                    (
                        Self::lua_to_string(&key)?,
                        Self::pretty_table(&sub, recursive, visited)?
                    )
                } else {
                    (
                        Self::lua_to_string(&key)?,
                        addr.clone(),
                    )
                }
            } else {
                (
                    Self::lua_to_string(&key)?,
                    Self::lua_to_string(&value)?,
                )
            };

            table.add_row(vec![key_str, value_str]);
        }

        if table.is_empty() {
            Ok(String::from("{}"))
        } else {
            Ok(table.to_string())
        }
    }

    fn handle_table(&self, tbl: &LuaTable) -> LuaResult<()> {
        match self.table_format {
            TableFormat::Address => println!("table@{:?}", tbl.to_pointer()),
            TableFormat::Inspect => {
                let inspect: LuaTable = self.lua.globals().get("_inspect")?;
                println!("{}", inspect.call::<String>(tbl)?);
            },
            TableFormat::ComfyTable(recursive) => {
                let mut visited = HashMap::new();
                println!("{}", Self::pretty_table(tbl, recursive, &mut visited)?)
            }
        }

        Ok(())
    }

    fn eval(&mut self, line: &str) -> LuaResult<()> {
        let value: LuaValue = self.lua.load(line).eval()?;

        match value {
            LuaValue::Table(tbl) => self.handle_table(&tbl)?,
            value => println!("{}", Self::lua_to_string(&value)?),
        }

        Ok(())
    }
}
