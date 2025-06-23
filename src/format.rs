use std::collections::HashMap;

use comfy_table::{Table, presets::UTF8_FULL_CONDENSED};
use mlua::prelude::*;
use nu_ansi_term::Color;
use reedline::Highlighter;

use crate::{highlight::LuaHighlighter, inspect::rewrite_types};

const INSPECT_CODE: &str = include_str!("inspect.lua");

pub enum TableFormat {
    ComfyTable(bool),
    Inspect,
    Address,
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

fn print_array(tbl: &LuaTable) -> String {
    let mut buff = Vec::new();

    for (_, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        if let LuaValue::Table(inner) = value {
            buff.push(print_array(&inner));
        } else {
            buff.push(rewrite_types(&value, true));
        }
    }

    format!("{{ {} }}", buff.join(", "))
}

fn comfy_table(
    tbl: &LuaTable,
    recursive: bool,
    visited: &mut HashMap<usize, usize>,
) -> LuaResult<String> {
    let addr = tbl.to_pointer() as usize;

    if let Some(id) = visited.get(&addr) {
        return Ok(format!("<table {id}>"));
    }

    let id = visited.len();
    visited.insert(addr, id);

    let (is_array, has_table) = is_array(tbl)?;

    if is_array && !has_table {
        return Ok(print_array(tbl));
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![format!("<table {id}>")]);

    for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        let (key_str, value_str) = if let LuaValue::Table(sub) = value {
            if recursive {
                (
                    rewrite_types(&key, false),
                    comfy_table(&sub, recursive, visited)?,
                )
            } else {
                (
                    rewrite_types(&key, false),
                    rewrite_types(&LuaValue::Table(sub), false),
                )
            }
        } else {
            (rewrite_types(&key, false), rewrite_types(&value, false))
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
    pub fn format(&self, lua: &Lua, tbl: &LuaTable, colorize: bool) -> LuaResult<String> {
        match self {
            TableFormat::Address => {
                if colorize {
                    Ok(format!(
                        "{}{}{}",
                        Color::LightBlue.paint("table"),
                        Color::Default.paint("@"),
                        Color::LightYellow.paint(format!("{:?}", tbl.to_pointer()))
                    ))
                } else {
                    Ok(format!("table@{:?}", tbl.to_pointer()))
                }
            }
            TableFormat::Inspect => {
                if let Some(inspect) = lua.globals().get::<Option<LuaTable>>("_inspect")? {
                    let out = inspect.call::<String>(tbl)?;

                    if colorize {
                        Ok(LuaHighlighter::new().highlight(&out, 0).render_simple())
                    } else {
                        Ok(out)
                    }
                } else {
                    let inspect: LuaTable = lua.load(INSPECT_CODE).eval()?;
                    lua.globals().set("_inspect", inspect)?;

                    self.format(lua, tbl, colorize)
                }
            }
            TableFormat::ComfyTable(recursive) => {
                let mut visited = HashMap::new();
                comfy_table(tbl, *recursive, &mut visited)
            }
        }
    }
}
