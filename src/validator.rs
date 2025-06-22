use mlua::prelude::*;
use nu_ansi_term::{Color, Style};
use reedline::{Hinter, History, ValidationResult, Validator};

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

    fn burner_lua() -> Lua {
        let lua = Lua::new_with(
            LuaStdLib::MATH | LuaStdLib::STRING | LuaStdLib::UTF8,
            LuaOptions::new(),
        )
        .unwrap();

        let math: LuaTable = lua.globals().get("math").unwrap();

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

        if let Ok(str) = value.to_string() {
            self.hint = str.clone();
            let style = Style::new().fg(Color::DarkGray);

            style.paint(format!(" ({str})")).to_string()
        } else {
            String::new()
        }
    }

    fn complete_hint(&self) -> String {
        String::new()
    }

    fn next_hint_token(&self) -> String {
        String::new()
    }
}
