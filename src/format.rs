use std::{collections::HashMap, sync::Arc};

use comfy_table::{Table, presets::UTF8_FULL_CONDENSED};
use mlua::prelude::*;
use nu_ansi_term::Color;

use crate::inspect::{display_basic, display_table, is_short_printable, print_array};

#[derive(Clone, Copy)]
pub enum TableFormat {
    ComfyTable(bool),
    Inspect,
    Address,
}

fn comfy_table_inner(
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

    let printable = is_short_printable(tbl);

    if printable {
        return Ok(print_array(tbl, false));
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![format!("<table {id}>")]);

    for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        let (key_str, value_str) = if let LuaValue::Table(sub) = value {
            if recursive {
                (
                    display_basic(&key, false),
                    comfy_table_inner(&sub, recursive, visited)?,
                )
            } else {
                (
                    display_basic(&key, false),
                    display_basic(&LuaValue::Table(sub), false),
                )
            }
        } else {
            (display_basic(&key, false), display_basic(&value, false))
        };

        table.add_row(vec![key_str, value_str]);
    }

    if table.is_empty() {
        Ok(String::from("{}"))
    } else {
        Ok(table.to_string())
    }
}

pub fn comfy_table(tbl: &LuaTable, recursive: bool) -> LuaResult<String> {
    let mut visited = HashMap::new();
    comfy_table_inner(tbl, recursive, &mut visited)
}

impl TableFormat {
    pub fn format(&self, tbl: &LuaTable, colorize: bool) -> LuaResult<String> {
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
                display_table(tbl, colorize).map_err(|e| LuaError::ExternalError(Arc::new(e)))
            }
            TableFormat::ComfyTable(recursive) => comfy_table(tbl, *recursive),
        }
    }
}
