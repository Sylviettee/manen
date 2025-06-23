use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Write},
    sync::Arc,
};

use aho_corasick::AhoCorasick;
use lazy_static::lazy_static;
use mlua::prelude::*;
use nu_ansi_term::{AnsiString, AnsiStrings, Color};

lazy_static! {
    static ref AC_REPLACEMENTS: (AhoCorasick, Vec<String>) = {
        let mut escapes = vec![
            String::from("\x07"),
            String::from("\x08"),
            String::from("\x0C"),
            String::from("\n"),
            String::from("\r"),
            String::from("\t"),
            String::from("\x0B"),
            String::from("\x7F"),
            String::from("\\"),
        ];

        let mut replacements = vec![
            String::from("\\a"),
            String::from("\\b"),
            String::from("\\f"),
            String::from("\\n"),
            String::from("\\r"),
            String::from("\\t"),
            String::from("\\v"),
            String::from("\\127"),
            String::from("\\\\"),
        ];

        for i in 0..=31 {
            escapes.push(String::from_utf8_lossy(&[i]).to_string());
            replacements.push(format!("\\{i}"));
        }

        (AhoCorasick::new(escapes).unwrap(), replacements)
    };
    static ref ESCAPER: &'static AhoCorasick = &AC_REPLACEMENTS.0;
    static ref REPLACEMENT_COLOR: Vec<String> = AC_REPLACEMENTS
        .1
        .iter()
        .map(|s| format!("{}{}", Color::Cyan.paint(s), Color::Green.prefix()))
        .collect();
    static ref KEYWORDS: HashSet<&'static str> = HashSet::from_iter([
        "and", "break", "do", "else", "elseif", "end", "else", "false", "for", "function", "goto",
        "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until",
        "while",
    ]);
}

fn escape_control(s: &str) -> String {
    ESCAPER
        .replace_all(s, &AC_REPLACEMENTS.1)
        .replace("\u{FFFD}", "\\x")
}

fn escape_control_color(s: &str) -> String {
    let s = ESCAPER.replace_all(s, &REPLACEMENT_COLOR);
    let mut chars = s.chars();
    let mut new = String::new();

    while let Some(c) = chars.next() {
        if c != '\u{FFFD}' {
            new.push(c);
            continue;
        }

        let (hex1, hex2) = (chars.next(), chars.next());
        let escape = format!(
            "\\x{}{}",
            hex1.unwrap_or_default(),
            hex2.unwrap_or_default()
        );

        new.push_str(&format!(
            "{}{}",
            Color::Cyan.paint(escape),
            Color::Green.prefix()
        ));
    }

    new
}

fn remove_invalid(mut bytes: &[u8]) -> String {
    let mut buffer = String::new();

    loop {
        match str::from_utf8(bytes) {
            Ok(s) => {
                buffer.push_str(s);
                return buffer;
            }
            Err(e) => {
                let (valid, invalid) = bytes.split_at(e.valid_up_to());

                if !valid.is_empty() {
                    //  SAFETY: We already know the bytes until this point are valid
                    buffer.push_str(unsafe { str::from_utf8_unchecked(valid) })
                }

                let error_len = if let Some(len) = e.error_len() {
                    len
                } else {
                    return buffer;
                };

                for bad_byte in &invalid[..error_len] {
                    // this *might* cause some false positives
                    buffer.push_str(&format!("\u{FFFD}{:X?}", bad_byte));
                }

                bytes = &invalid[error_len..];
            }
        }
    }
}

pub fn cleanup_string(lua_str: &LuaString) -> String {
    escape_control(&remove_invalid(&lua_str.as_bytes()))
}

fn format_string(lua_str: &LuaString, colorize: bool) -> String {
    let mut s = remove_invalid(&lua_str.as_bytes());

    if colorize {
        s = escape_control_color(&s);
    } else {
        s = escape_control(&s);
    }

    let pair = (s.contains("'"), s.contains('"'));

    match pair {
        (true, true) => format!("\"{}\"", s.replace("\"", "\\\"")),
        (false, true) => format!("'{s}'"),
        (true, false) | (false, false) => format!("\"{s}\""),
    }
}

fn addr_color(value: &LuaValue) -> Option<(String, Color)> {
    match value {
        LuaValue::LightUserData(l) => Some((format!("{:?}", l.0), Color::Cyan)),
        LuaValue::Table(t) => Some((format!("{:?}", t.to_pointer()), Color::LightBlue)),
        LuaValue::Function(f) => Some((format!("{:?}", f.to_pointer()), Color::Purple)),
        LuaValue::Thread(t) => Some((format!("{:?}", t.to_pointer()), Color::LightGray)),
        LuaValue::UserData(u) => Some((format!("{:?}", u.to_pointer()), Color::Cyan)),
        _ => None,
    }
}

fn handle_strings<'a>(colorize: bool, strings: AnsiStrings<'a>) -> String {
    if colorize {
        strings.to_string()
    } else {
        nu_ansi_term::unstyle(&strings)
    }
}

pub fn display_basic(value: &LuaValue, colorize: bool) -> String {
    match addr_color(value) {
        Some((addr, color)) => {
            let strings: &[AnsiString<'static>] = &[
                color.paint(value.type_name()),
                Color::Default.paint("@"),
                Color::LightYellow.paint(addr),
            ];

            handle_strings(colorize, AnsiStrings(strings))
        }
        None => {
            let strings = &[match value {
                LuaValue::Nil => Color::LightRed.paint("nil"),
                LuaValue::Boolean(b) => Color::LightYellow.paint(b.to_string()),
                LuaValue::Integer(i) => Color::LightYellow.paint(i.to_string()),
                LuaValue::Number(n) => Color::LightYellow.paint(n.to_string()),
                LuaValue::String(s) => Color::Green.paint(format_string(s, colorize)),
                #[cfg(feature = "luau")]
                LuaValue::Vector(v) => {
                    let strings: &[AnsiString<'static>] = &[
                        Color::Default.paint("<"),
                        Color::LightYellow.paint(v.x().to_string()),
                        Color::Default.paint(", "),
                        Color::LightYellow.paint(v.y().to_string()),
                        Color::Default.paint(", "),
                        Color::LightYellow.paint(v.z().to_string()),
                        #[cfg(feature = "luau-vector4")]
                        Color::Default.paint(", "),
                        #[cfg(feature = "luau-vector4")]
                        Color::LightYellow.paint(v.w().to_string()),
                        Color::Default.paint(">"),
                    ];

                    return handle_strings(colorize, AnsiStrings(strings));
                }
                val => Color::LightGray.paint(val.to_string().unwrap_or_default()),
            }];

            handle_strings(colorize, AnsiStrings(strings))
        }
    }
}

fn is_short_printable_inner(tbl: &LuaTable, seen: &mut HashSet<usize>) -> bool {
    let addr = tbl.to_pointer() as usize;

    if seen.contains(&addr) {
        return false;
    }

    seen.insert(addr);

    for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        if !key.is_integer() {
            return false;
        }

        if let LuaValue::Table(inner) = value {
            let printable = is_short_printable_inner(&inner, seen);

            if !printable {
                return false;
            }
        }
    }

    true
}

pub fn is_short_printable(tbl: &LuaTable) -> bool {
    let mut seen = HashSet::new();

    is_short_printable_inner(tbl, &mut seen)
}

pub fn print_array(tbl: &LuaTable, colorize: bool) -> String {
    let mut buff = Vec::new();

    if tbl.is_empty() {
        return String::from("{}");
    }

    for (_, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        if let LuaValue::Table(inner) = value {
            buff.push(print_array(&inner, colorize));
        } else {
            buff.push(display_basic(&value, colorize));
        }
    }

    format!("{{ {} }}", buff.join(", "))
}

fn is_valid_identifier(s: &str) -> bool {
    if KEYWORDS.contains(s) {
        return false;
    }

    let mut chars = s.chars();
    let first = if let Some(c) = chars.next() {
        c
    } else {
        return false;
    };

    //  [A-Z_a-z]
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    let s = chars.as_str();

    if s.is_empty() {
        return true;
    }

    // [A-Z_a-z0-9]
    if s.find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .is_some()
    {
        return false;
    }

    true
}

fn display_table_inner(
    tbl: &LuaTable,
    colorize: bool,
    seen: &mut HashMap<usize, usize>,
    indent: usize,
) -> Result<String, fmt::Error> {
    let ptr = tbl.to_pointer() as usize;
    if let Some(id) = seen.get(&ptr) {
        return Ok(format!("<{id}>"));
    }

    let id = seen.len();
    seen.insert(ptr, id);

    let printable = is_short_printable(tbl);

    if printable {
        return Ok(print_array(tbl, colorize));
    }

    let mut buffer = String::new();

    // TODO; only output id if necessary
    writeln!(&mut buffer, "<{id}>{{")?;

    for (key, value) in tbl.pairs::<LuaValue, LuaValue>().flatten() {
        buffer.push_str(&("   ".repeat(indent + 1)));

        if let LuaValue::String(ref s) = key {
            let clean = cleanup_string(s);

            if is_valid_identifier(&clean) {
                write!(&mut buffer, "{clean} = ")?
            } else {
                write!(&mut buffer, "[{}] = ", display_basic(&key, colorize))?
            }
        } else {
            write!(&mut buffer, "[{}] = ", display_basic(&key, colorize))?;
        }

        if let LuaValue::Table(t) = value {
            writeln!(
                &mut buffer,
                "{},",
                display_table_inner(&t, colorize, seen, indent + 1)?
            )?;
        } else {
            writeln!(&mut buffer, "{},", display_basic(&value, colorize))?;
        }
    }

    write!(&mut buffer, "{}}}", "   ".repeat(indent))?;

    Ok(buffer)
}

pub fn display_table(tbl: &LuaTable, colorize: bool) -> Result<String, fmt::Error> {
    let mut seen = HashMap::new();

    display_table_inner(tbl, colorize, &mut seen, 0)
}

pub fn inspect(value: &LuaValue, colorize: bool) -> LuaResult<String> {
    match value {
        LuaValue::Table(tbl) => {
            display_table(tbl, colorize).map_err(|e| LuaError::ExternalError(Arc::new(e)))
        }
        value => Ok(display_basic(value, colorize)),
    }
}
