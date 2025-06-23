use mlua::prelude::*;
use nu_ansi_term::{Color, Style};
use reedline::{Hinter, History, ValidationResult, Validator};

use crate::inspect::display_basic;

pub struct LuaValidator {
    lua: Lua,
    hint: String,
}

impl LuaValidator {
    pub fn new() -> Self {
        Self {
            lua: Self::burner_lua(),
            hint: String::new(),
        }
    }

    // this is a really bad way of doing things
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
}

fn load_lua(lua: &Lua, code: &str) -> LuaResult<LuaFunction> {
    if let Ok(func) = lua.load(format!("return ({code})")).into_function() {
        return Ok(func);
    }

    lua.load(code).into_function()
}

impl Validator for LuaValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        if line.starts_with(".") {
            return ValidationResult::Complete;
        }

        match load_lua(&self.lua, line) {
            Ok(_) => ValidationResult::Complete,
            Err(LuaError::SyntaxError {
                incomplete_input, ..
            }) => {
                if incomplete_input {
                    ValidationResult::Incomplete
                } else {
                    ValidationResult::Complete
                }
            }
            Err(_) => ValidationResult::Complete,
        }
    }
}

impl Hinter for LuaValidator {
    fn handle(
        &mut self,
        line: &str,
        _pos: usize,
        _history: &dyn History,
        _use_ansi_coloring: bool,
        _cwd: &str,
    ) -> String {
        let lua = Self::burner_lua();

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

        self.hint = display_basic(&value, false);
        let style = Style::new().fg(Color::DarkGray);

        style.paint(format!(" ({})", &self.hint)).to_string()
    }

    fn complete_hint(&self) -> String {
        String::new()
    }

    fn next_hint_token(&self) -> String {
        String::new()
    }
}
