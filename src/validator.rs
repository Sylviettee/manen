use mlua::prelude::*;
use reedline::{ValidationResult, Validator};

pub struct LuaValidator {
    lua: Lua,
}

impl LuaValidator {
    pub fn new() -> Self {
        Self { lua: Lua::new() }
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
