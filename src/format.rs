use std::collections::HashMap;

use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use mlua::prelude::*;

const INSPECT_CODE: &str = include_str!("inspect.lua");

pub enum TableFormat {
    ComfyTable(bool),
    Inspect,
    Address,
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

pub fn lua_to_string(value: &LuaValue) -> LuaResult<String> {
    match value {
        LuaValue::String(string) => Ok(convert_string(string)),
        LuaValue::Table(tbl) => Ok(addr_tbl(tbl)),
        value => value.to_string()
    }
}

fn is_array(tbl: &LuaTable) -> LuaResult<(bool, bool)> {
    let mut is_arr = true;
    let mut has_table = false;

    for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        if !(key.is_integer() || key.is_number()) {
            is_arr = false;
        }

        if let LuaValue::Table(inner) = value {
            let (is_arr, has_tbl) = is_array(&inner)?;

            if !is_arr || has_tbl {
                has_table = true;
            }
        }
    }

    Ok((is_arr, has_table))
}

fn print_array(tbl: &LuaTable) -> LuaResult<String> {
    let mut buff = Vec::new();

    for (_, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        if let LuaValue::Table(inner) = value {
            buff.push(print_array(&inner)?);
        } else {
            buff.push(lua_to_string(&value)?);
        }
    }

    Ok(format!("{{ {} }}", buff.join(", ")))
}

fn comfy_table(tbl: &LuaTable, recursive: bool, visited: &mut HashMap<String, usize>) -> LuaResult<String> {
    let addr = addr_tbl(tbl);

    if let Some(id) = visited.get(&addr) {
        return Ok(format!("<table {id}>"))
    }
    
    let id = visited.len();
    visited.insert(addr.clone(), id);

    let (is_array, has_table) = is_array(tbl)?;

    if is_array && !has_table {
        return print_array(tbl)
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![format!("<table {id}>")]);

    for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        let (key_str, value_str) = if let LuaValue::Table(sub) = value {
            if recursive {
                (
                    lua_to_string(&key)?,
                    comfy_table(&sub, recursive, visited)?
                )
            } else {
                (
                    lua_to_string(&key)?,
                    addr.clone(),
                )
            }
        } else {
            (
                lua_to_string(&key)?,
                lua_to_string(&value)?,
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

impl TableFormat {
    pub fn format(&self, lua: &Lua, tbl: &LuaTable) -> LuaResult<String> {
        match self {
            TableFormat::Address => Ok(format!("table@{:?}", tbl.to_pointer())),
            TableFormat::Inspect => {
                if let Some(inspect) = lua.globals().get::<Option<LuaTable>>("_inspect")? {
                    inspect.call::<String>(tbl)
                } else {
                    let inspect: LuaTable = lua.load(INSPECT_CODE).eval()?;
                    lua.globals().set("_inspect", inspect)?;

                    self.format(lua, tbl)
                }
            },
            TableFormat::ComfyTable(recursive) => {
                let mut visited = HashMap::new();
                comfy_table(tbl, *recursive, &mut visited)
            }
        }
    }
}
