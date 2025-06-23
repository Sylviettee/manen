use aho_corasick::AhoCorasick;
use lazy_static::lazy_static;
use mlua::prelude::*;

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
        ];

        for i in 0..=31 {
            escapes.push(String::from_utf8_lossy(&[i]).to_string());
            replacements.push(format!("\\{i}"));
        }

        (AhoCorasick::new(escapes).unwrap(), replacements)
    };
    static ref ESCAPER: &'static AhoCorasick = &AC_REPLACEMENTS.0;
}

fn escape_control(s: &str) -> String {
    ESCAPER
        .replace_all(s, &AC_REPLACEMENTS.1)
        .replace("\\\\x", "\\x")
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
