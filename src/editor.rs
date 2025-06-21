use std::collections::HashSet;

use comfy_table::{presets::UTF8_FULL, Table};
use mlua::prelude::*;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crate::highlight::LuaHighlighter;

pub enum TableFormat {
    Pretty,
    Lua,
    Address,
}

pub struct Editor {
    prompt: DefaultPrompt,
    editor: Reedline,
    lua: Lua,

    table_format: TableFormat,
    print_nested_tables: bool,
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

            table_format: TableFormat::Pretty,
            print_nested_tables: true,
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

    fn pretty_table(&self, tbl: &LuaTable, visited: &mut HashSet<String>) -> LuaResult<String> {
        let addr = Self::addr_tbl(tbl);

        if visited.contains(&addr) {
            return Ok(format!("{addr} (self-reference)"))
        } else {
            visited.insert(addr.clone());
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);

        for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
            let value_str = if let LuaValue::Table(sub) = value {
                if self.print_nested_tables {
                    self.pretty_table(&sub, visited)?
                } else {
                    addr.clone()
                }
            } else {
                Self::lua_to_string(&value)?
            };

            table.add_row(vec![Self::lua_to_string(&key)?, value_str]);
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
            TableFormat::Lua => todo!(),
            TableFormat::Pretty => {
                let mut visited = HashSet::new();
                println!("{}", self.pretty_table(tbl, &mut visited)?)
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
