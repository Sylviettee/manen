use mlua::prelude::*;
use nu_ansi_term::{Color, Style};
use reedline::{Hinter, History};

use crate::inspect::display_basic;

fn burner_lua() -> Lua {
    #[cfg(any(feature = "lua54", feature = "lua53"))]
    let flags = LuaStdLib::MATH | LuaStdLib::STRING | LuaStdLib::UTF8;
    #[cfg(not(any(feature = "lua54", feature = "lua53")))]
    let flags = LuaStdLib::MATH | LuaStdLib::STRING;

    let lua = Lua::new_with(flags, LuaOptions::new()).unwrap();

    let globals = lua.globals();
    globals.raw_remove("print").unwrap();
    globals.raw_remove("loadfile").unwrap();
    globals.raw_remove("load").unwrap();

    let math: LuaTable = globals.get("math").unwrap();
    math.raw_remove("random").unwrap();

    lua
}

pub struct LuaHinter;

impl Hinter for LuaHinter {
    fn handle(
        &mut self,
        line: &str,
        _pos: usize,
        _history: &dyn History,
        _use_ansi_coloring: bool,
        _cwd: &str,
    ) -> String {
        let lua = burner_lua();

        let value: LuaValue = match lua.load(line).set_name("=").eval() {
            Ok(value) => value,
            Err(LuaError::SyntaxError { message, .. }) => {
                let message = message.split(":").last().unwrap().trim();
                let style = Style::new().fg(Color::Red).dimmed();

                return style.paint(format!(" ({message})")).to_string();
            }
            Err(_) => return String::new(),
        };

        if value.is_nil() {
            return String::new();
        }

        let style = Style::new().fg(Color::DarkGray);

        style
            .paint(format!(" ({})", display_basic(&value, false)))
            .to_string()
    }

    fn complete_hint(&self) -> String {
        String::new()
    }

    fn next_hint_token(&self) -> String {
        String::new()
    }
}
