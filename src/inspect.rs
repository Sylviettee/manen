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
}

fn escape_control(s: &str) -> String {
    ESCAPER
        .replace_all(s, &AC_REPLACEMENTS.1)
        .replace("\\\\x", "\\x")
}

fn escape_control_color(s: &str) -> String {
    ESCAPER.replace_all(s, &REPLACEMENT_COLOR).replace(
        "\\\\x",
        &format!("{}{}", Color::Cyan.paint("\\x"), Color::Green.prefix()),
    )
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
                    buffer.push_str(&format!("\\x{:X?}", bad_byte));
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

fn addr_color(val: &LuaValue) -> Option<(String, Color)> {
    match val {
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

pub fn rewrite_types(val: &LuaValue, colorize: bool) -> String {
    match addr_color(val) {
        Some((addr, color)) => {
            let strings: &[AnsiString<'static>] = &[
                color.paint(val.type_name()),
                Color::Default.paint("@"),
                Color::LightYellow.paint(addr),
            ];

            handle_strings(colorize, AnsiStrings(strings))
        }
        None => {
            let strings = &[match val {
                LuaValue::Nil => Color::LightRed.paint("nil"),
                LuaValue::Boolean(b) => Color::LightYellow.paint(b.to_string()),
                LuaValue::Integer(i) => Color::LightYellow.paint(i.to_string()),
                LuaValue::Number(n) => Color::LightYellow.paint(n.to_string()),
                LuaValue::String(s) => Color::Green.paint(format_string(s, colorize)),
                val => Color::LightGray.paint(val.to_string().unwrap_or_default()),
            }];

            handle_strings(colorize, AnsiStrings(strings))
        }
    }
}
