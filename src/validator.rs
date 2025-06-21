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

impl Validator for LuaValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        match self.lua.load(line).into_function() {
            Ok(_) => ValidationResult::Complete,
            Err(_) => ValidationResult::Incomplete,
        }
    }
}
