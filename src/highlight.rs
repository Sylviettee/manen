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
    "attribute",
    "function",
    "function.call",
    "function.builtin",
    "method",
    "method.call",
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
    "preproc",
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
    style_fg(Color::Purple),      // keyword
    style_fg(Color::Purple),      // keyword.return
    style_fg(Color::Purple),      // keyword.function
    style_fg(Color::Purple),      // keyword.operator
    style_fg(Color::LightGray),   // punctuation.delimiter
    style_fg(Color::LightRed),    // punctuation.bracket
    style_fg(Color::LightGray),   // variable
    style_fg(Color::LightRed),    // variable.builtin
    style_fg(Color::LightYellow), // constant
    style_fg(Color::LightYellow), // constant.builtin
    style_fg(Color::Red),         // attribute
    style_fg(Color::LightBlue),   // function
    style_fg(Color::LightBlue),   // function.call
    style_fg(Color::LightBlue),   // function.builtin
    style_fg(Color::LightBlue),   // method
    style_fg(Color::LightBlue),   // method.call
    style_fg(Color::Red),         // parameter
    style_fg(Color::Green),       // string
    style_fg(Color::Cyan),        // string.escape
    style_fg(Color::LightYellow), // boolean
    style_fg(Color::LightYellow), // number
    style_fg(Color::LightGray),   // field
    style_fg(Color::LightRed),    // constructor
    style_fg(Color::LightGray),   // label
    style_fg(Color::Purple),      // repeat
    style_fg(Color::Purple),      // conditional
    style_fg(Color::LightBlue),   // operator
    style_fg(Color::DarkGray),    // comment
    style_fg(Color::DarkGray),    // preproc
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
            tree_sitter_lua::LOCALS_QUERY,
        )
        .unwrap();

        config.configure(LUA_HIGHLIGHT_NAMES);

        Self {
            highlighter: RefCell::new(highlighter),
            config,
        }
    }
}

impl reedline::Highlighter for LuaHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut binding = self.highlighter.borrow_mut();
        let highlights = binding.highlight(&self.config, line.as_bytes(), None, |_| None);

        let mut text = StyledText::new();

        let highlights = if let Ok(highlights) = highlights {
            highlights
        } else {
            text.push((Style::new(), line.to_string()));

            return text;
        };

        let mut style = Style::new();
        let mut highlight = 0usize;

        for event in highlights.flatten() {
            match event {
                HighlightEvent::Source { start, end } => {
                    text.push((style, line[start..end].to_string()));

                    if highlight == 18 {
                        style = STYLES[17];
                    }
                }
                HighlightEvent::HighlightStart(s) => {
                    style = STYLES[s.0];
                    highlight = s.0;
                }
                HighlightEvent::HighlightEnd => {}
            }
        }

        text
    }
}
