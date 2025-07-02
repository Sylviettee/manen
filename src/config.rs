use directories::ProjectDirs;
use mlua::prelude::*;
use std::{path::PathBuf, sync::Arc};

use crate::{
    inspect::TableFormat,
    lua::{LuaExecutor, MluaExecutor, SystemLuaError, SystemLuaExecutor},
};

#[derive(Clone, Copy)]
pub enum Executor {
    System,
    Embedded,
}

#[derive(Clone, FromLua)]
pub struct Config {
    pub executor: Executor,
    pub system_lua: Option<PathBuf>,
    pub table_format: TableFormat,
    pub history_size: usize,
    pub color_output: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            executor: Executor::Embedded,
            system_lua: None,
            table_format: TableFormat::Inspect,
            history_size: 256,
            color_output: true,
        }
    }
}

impl Config {
    pub fn load() -> LuaResult<Self> {
        let config = Self::default();

        if let Some(proj_dirs) = ProjectDirs::from("gay.gayest", "", "Manen") {
            let config_file = proj_dirs.config_dir().join("config.lua");

            if !config_file.exists() {
                return Ok(config);
            }

            let lua = Lua::new();

            lua.globals().set("manen", config)?;
            lua.load(config_file).exec()?;

            return lua.globals().get("manen");
        }

        Ok(config)
    }

    pub fn get_executor(&self) -> Result<Arc<dyn LuaExecutor>, SystemLuaError> {
        match self.executor {
            Executor::Embedded => Ok(Arc::new(MluaExecutor::new())),
            Executor::System => {
                if let Some(path) = &self.system_lua {
                    Ok(Arc::new(SystemLuaExecutor::new(&path.to_string_lossy())?))
                } else {
                    Ok(Arc::new(MluaExecutor::new()))
                }
            }
        }
    }
}

macro_rules! field {
    ($value:ident, $as_field:ident, $field:expr, $expected:expr) => {
        $value.$as_field().ok_or_else(|| {
            LuaError::RuntimeError(format!(
                "invalid type '{}' for {}, expected {}",
                $value.type_name(),
                $field,
                $expected,
            ))
        })?
    };
}

impl LuaUserData for Config {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |lua, this, (key, value): (String, LuaValue)| {
                match key.as_str() {
                    "executor" => {
                        let executor = field!(value, as_string_lossy, "executor", "string");

                        match executor.as_str() {
                            "system" => this.executor = Executor::System,
                            "embedded" => this.executor = Executor::Embedded,
                            _ => {
                                return Err(LuaError::RuntimeError(String::from(
                                    "expected valid executor format",
                                )));
                            }
                        }
                    }
                    "system_lua" => {
                        if value.is_nil() {
                            this.system_lua = None;
                            return Ok(());
                        }

                        let path = PathBuf::from_lua(value, lua)?;

                        if !path.exists() {
                            return Err(LuaError::RuntimeError(format!(
                                "path '{}' does not exist",
                                path.to_string_lossy()
                            )));
                        }

                        this.system_lua = Some(path);
                    }
                    "table_format" => {
                        let format = field!(value, as_string_lossy, "table_format", "string");

                        match format.as_str() {
                            "address" => this.table_format = TableFormat::Address,
                            "inspect" => this.table_format = TableFormat::Inspect,
                            "comfytable" => this.table_format = TableFormat::ComfyTable,
                            _ => {
                                return Err(LuaError::RuntimeError(String::from(
                                    "expected valid table format",
                                )));
                            }
                        }
                    }
                    "history_size" => {
                        this.history_size = field!(value, as_usize, "history_size", "integer");
                    }
                    "color_output" => {
                        this.color_output = field!(value, as_boolean, "color_output", "bool");
                    }
                    key => return Err(LuaError::RuntimeError(format!("invalid key '{key}'"))),
                }
                Ok(())
            },
        );
    }
}
