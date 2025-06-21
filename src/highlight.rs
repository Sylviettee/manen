use std::cell::RefCell;

use nu_ansi_term::{Color, Style};
use reedline::StyledText;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};

const LUA_HIGHLIGHT_NAMES: &[&str] = &[
    "keyword",
    "keyword.return",
    "keyword.function",
    "keyword.operator",

    "punctuation.delimiter",
    "punctuation.bracket",

    "variable",
    "variable.builtin",

    "constant",
    "constant.builtin",

    "function",
    "function.call",
    "function.builtin",
    "method",
    "parameter",

    "string",
    "string.escape",
    "boolean",
    "number",

    "field",
    "constructor",

    "label",
    "repeat",
    "conditional",
    "operator",
    "comment",
];

const fn style_fg(color: Color) -> Style {
    Style {
        foreground: Some(color),
        background: None,
        is_bold: false,
        is_dimmed: false,
        is_italic: false,
        is_underline: false,
        is_blink: false,
        is_reverse: false,
        is_hidden: false,
        is_strikethrough: false,
        prefix_with_reset: true,
    }
}

const STYLES: &[Style] = &[
    style_fg(Color::Purple),
    style_fg(Color::Purple),
    style_fg(Color::Purple),
    style_fg(Color::Purple),

    style_fg(Color::LightGray),
    style_fg(Color::LightRed),

    style_fg(Color::LightGray),
    style_fg(Color::Red),

    style_fg(Color::Magenta),
    style_fg(Color::Magenta),

    style_fg(Color::LightBlue),
    style_fg(Color::LightBlue),
    style_fg(Color::LightBlue),
    style_fg(Color::LightBlue),
    style_fg(Color::LightRed),

    style_fg(Color::Green),
    style_fg(Color::Cyan),
    style_fg(Color::Yellow),
    style_fg(Color::Yellow),

    style_fg(Color::LightGray),
    style_fg(Color::LightRed),

    style_fg(Color::LightGray),
    style_fg(Color::Purple),
    style_fg(Color::Purple),
    style_fg(Color::LightBlue),
    style_fg(Color::DarkGray),
];

pub struct LuaHighlighter {
    highlighter: RefCell<tree_sitter_highlight::Highlighter>,
    config: HighlightConfiguration,
}

impl LuaHighlighter {
    pub fn new() -> Self {
        let highlighter = tree_sitter_highlight::Highlighter::new();
        let mut config = HighlightConfiguration::new(
            tree_sitter_lua::LANGUAGE.into(),
            "lua",
            tree_sitter_lua::HIGHLIGHTS_QUERY,
            tree_sitter_lua::INJECTIONS_QUERY,
            tree_sitter_lua::LOCALS_QUERY
        ).unwrap();

        config.configure(LUA_HIGHLIGHT_NAMES);

        Self {
            highlighter: RefCell::new(highlighter),
            config
        }
    }
}

impl reedline::Highlighter for LuaHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut binding = self.highlighter.borrow_mut();
        let highlights = binding.highlight(
            &self.config,
            line.as_bytes(),
            None,
            |_| None
        );

        let mut text = StyledText::new();

        let highlights = if let Ok(highlights) = highlights {
            highlights
        } else {
            text.push((Style::new(), line.to_string()));

            return text
        };

        let mut style = Style::new();

        for event in highlights.flatten() {
            match event {
                HighlightEvent::Source {start, end} => {
                    text.push((style,  line[start..end].to_string()))
                },
                HighlightEvent::HighlightStart(s) => {
                    style = STYLES[s.0];
                },
                HighlightEvent::HighlightEnd => {}
            }
        }

        text
    }
}
