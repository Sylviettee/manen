use emmylua_parser::{
    LuaAst, LuaAstNode, LuaKind, LuaParser, LuaSyntaxKind, LuaSyntaxNode, LuaSyntaxToken,
    LuaSyntaxTree, LuaTokenKind, ParserConfig,
};
use nu_ansi_term::{Color, Style};
use reedline::StyledText;
use rowan::WalkEvent;

fn node_name(node: &LuaAst) -> Option<&'static str> {
    match node {
        LuaAst::LuaChunk(_) => Some("chunk"),
        LuaAst::LuaBlock(_) => Some("block"),
        LuaAst::LuaAssignStat(_) => Some("assignment"),
        LuaAst::LuaLocalStat(_) => Some("local"),
        LuaAst::LuaCallExprStat(_) => Some("call_statement"),
        LuaAst::LuaLabelStat(_) => Some("label"),
        LuaAst::LuaBreakStat(_) => Some("break"),
        LuaAst::LuaGotoStat(_) => Some("goto"),
        LuaAst::LuaDoStat(_) => Some("do"),
        LuaAst::LuaWhileStat(_) => Some("while"),
        LuaAst::LuaRepeatStat(_) => Some("repeat"),
        LuaAst::LuaIfStat(_) => Some("if"),
        LuaAst::LuaForStat(_) => Some("for_range"),
        LuaAst::LuaForRangeStat(_) => Some("for"),
        LuaAst::LuaFuncStat(_) => Some("function"),
        LuaAst::LuaLocalFuncStat(_) => Some("local_function"),
        LuaAst::LuaReturnStat(_) => Some("return"),
        LuaAst::LuaNameExpr(_) => Some("identifier"),
        LuaAst::LuaIndexExpr(_) => Some("index"),
        LuaAst::LuaTableExpr(_) => Some("table"),
        LuaAst::LuaBinaryExpr(_) => Some("binop"),
        LuaAst::LuaUnaryExpr(_) => Some("unop"),
        LuaAst::LuaParenExpr(_) => Some("parenthesis"),
        LuaAst::LuaCallExpr(_) => Some("call"),
        LuaAst::LuaLiteralExpr(_) => Some("literal"),
        LuaAst::LuaClosureExpr(_) => Some("closure"),
        LuaAst::LuaTableField(_) => Some("table_field"),
        LuaAst::LuaParamList(_) => Some("parameters"),
        LuaAst::LuaParamName(_) => Some("parameter"),
        LuaAst::LuaCallArgList(_) => Some("arguments"),
        LuaAst::LuaLocalName(_) => Some("identifier"),
        LuaAst::LuaLocalAttribute(_) => Some("attribute"),
        LuaAst::LuaElseIfClauseStat(_) => Some("elseif"),
        LuaAst::LuaElseClauseStat(_) => Some("else"),
        LuaAst::LuaComment(_) => Some("comment"),
        _ => None,
    }
}

fn should_print_contents(node: &LuaAst) -> bool {
    matches!(
        node,
        LuaAst::LuaLabelStat(_)
            | LuaAst::LuaGotoStat(_)
            | LuaAst::LuaNameExpr(_)
            | LuaAst::LuaIndexExpr(_)
            | LuaAst::LuaTableExpr(_)
            | LuaAst::LuaBinaryExpr(_)
            | LuaAst::LuaUnaryExpr(_)
            | LuaAst::LuaParenExpr(_)
            | LuaAst::LuaCallExpr(_)
            | LuaAst::LuaLiteralExpr(_)
            | LuaAst::LuaTableField(_)
            | LuaAst::LuaParamName(_)
            | LuaAst::LuaLocalName(_)
            | LuaAst::LuaLocalAttribute(_)
            | LuaAst::LuaComment(_)
    )
}

pub fn debug_tree(tree: &LuaSyntaxTree) {
    let chunk = tree.get_chunk_node();
    let mut depth = -1isize;

    for event in chunk.walk_descendants::<LuaAst>() {
        match event {
            WalkEvent::Enter(node) => {
                if let Some(name) = node_name(&node) {
                    depth += 1;

                    let syntax = node.syntax();
                    let range = syntax.text_range();
                    let start: u32 = range.start().into();
                    let end: u32 = range.end().into();

                    let text = if should_print_contents(&node) {
                        format!("`{}`", syntax.text())
                    } else {
                        String::new()
                    };

                    println!(
                        "{}{name} [{start}-{end}] {}",
                        "   ".repeat(depth as usize),
                        text
                    )
                }
            }
            WalkEvent::Leave(_) => {
                depth -= 1;
            }
        }
    }
}

fn default_token_color(token: &LuaSyntaxToken) -> Color {
    let kind = match token.kind() {
        LuaKind::Syntax(_) => unreachable!(),
        LuaKind::Token(kind) => kind,
    };

    match kind {
        LuaTokenKind::TkWhitespace
        | LuaTokenKind::TkEndOfLine
        | LuaTokenKind::TkEof
        | LuaTokenKind::TkUnknown
        | LuaTokenKind::None => Color::Default,

        LuaTokenKind::TkBreak
        | LuaTokenKind::TkDo
        | LuaTokenKind::TkElse
        | LuaTokenKind::TkElseIf
        | LuaTokenKind::TkEnd
        | LuaTokenKind::TkFor
        | LuaTokenKind::TkFunction
        | LuaTokenKind::TkGoto
        | LuaTokenKind::TkIf
        | LuaTokenKind::TkIn
        | LuaTokenKind::TkLocal
        | LuaTokenKind::TkRepeat
        | LuaTokenKind::TkReturn
        | LuaTokenKind::TkThen
        | LuaTokenKind::TkUntil
        | LuaTokenKind::TkWhile
        | LuaTokenKind::TkGlobal => Color::Purple,

        LuaTokenKind::TkOr | LuaTokenKind::TkNot | LuaTokenKind::TkAnd => Color::Cyan,

        LuaTokenKind::TkFalse | LuaTokenKind::TkTrue | LuaTokenKind::TkNil => Color::Red,

        LuaTokenKind::TkInt | LuaTokenKind::TkFloat | LuaTokenKind::TkComplex => Color::LightYellow,

        LuaTokenKind::TkPlus
        | LuaTokenKind::TkMinus
        | LuaTokenKind::TkMul
        | LuaTokenKind::TkDiv
        | LuaTokenKind::TkIDiv
        | LuaTokenKind::TkDot
        | LuaTokenKind::TkConcat
        | LuaTokenKind::TkDots
        | LuaTokenKind::TkComma
        | LuaTokenKind::TkAssign
        | LuaTokenKind::TkEq
        | LuaTokenKind::TkGe
        | LuaTokenKind::TkLe
        | LuaTokenKind::TkNe
        | LuaTokenKind::TkShl
        | LuaTokenKind::TkShr
        | LuaTokenKind::TkLt
        | LuaTokenKind::TkGt
        | LuaTokenKind::TkMod
        | LuaTokenKind::TkPow
        | LuaTokenKind::TkLen
        | LuaTokenKind::TkBitAnd
        | LuaTokenKind::TkBitOr
        | LuaTokenKind::TkBitXor
        | LuaTokenKind::TkColon
        | LuaTokenKind::TkDbColon
        | LuaTokenKind::TkSemicolon
        | LuaTokenKind::TkLeftBracket
        | LuaTokenKind::TkRightBracket
        | LuaTokenKind::TkLeftParen
        | LuaTokenKind::TkRightParen
        | LuaTokenKind::TkLeftBrace
        | LuaTokenKind::TkRightBrace => Color::LightGray,

        LuaTokenKind::TkName => Color::LightGray,

        LuaTokenKind::TkString | LuaTokenKind::TkLongString => Color::Green,

        LuaTokenKind::TkShortComment | LuaTokenKind::TkLongComment | LuaTokenKind::TkShebang => {
            Color::DarkGray
        }

        // EmmyLua
        LuaTokenKind::TkTagClass
        | LuaTokenKind::TkTagEnum
        | LuaTokenKind::TkTagInterface
        | LuaTokenKind::TkTagAlias
        | LuaTokenKind::TkTagModule
        | LuaTokenKind::TkTagField
        | LuaTokenKind::TkTagType
        | LuaTokenKind::TkTagParam
        | LuaTokenKind::TkTagReturn
        | LuaTokenKind::TkTagOverload
        | LuaTokenKind::TkTagGeneric
        | LuaTokenKind::TkTagSee
        | LuaTokenKind::TkTagDeprecated
        | LuaTokenKind::TkTagAsync
        | LuaTokenKind::TkTagCast
        | LuaTokenKind::TkTagOther
        | LuaTokenKind::TkTagVisibility
        | LuaTokenKind::TkTagReadonly
        | LuaTokenKind::TkTagDiagnostic
        | LuaTokenKind::TkTagMeta
        | LuaTokenKind::TkTagVersion
        | LuaTokenKind::TkTagAs
        | LuaTokenKind::TkTagNodiscard
        | LuaTokenKind::TkTagOperator
        | LuaTokenKind::TkTagMapping
        | LuaTokenKind::TkTagNamespace
        | LuaTokenKind::TkTagUsing
        | LuaTokenKind::TkTagSource
        | LuaTokenKind::TkTagReturnCast => Color::LightMagenta,
        LuaTokenKind::TkDocVisibility => Color::Purple,
        _ => Color::DarkGray,
    }
}

fn modify_token_color(token: &LuaSyntaxToken, parent: &LuaSyntaxNode) -> Option<Color> {
    let tk_kind = match token.kind() {
        LuaKind::Syntax(_) => unreachable!(),
        LuaKind::Token(kind) => kind,
    };

    let node_kind = match parent.kind() {
        LuaKind::Syntax(kind) => kind,
        LuaKind::Token(_) => unreachable!(),
    };

    match (tk_kind, node_kind) {
        (LuaTokenKind::TkName, LuaSyntaxKind::TypeName) => Some(Color::Yellow),
        (LuaTokenKind::TkName, LuaSyntaxKind::DocTagParam) => Some(Color::Red),
        (LuaTokenKind::TkName, LuaSyntaxKind::ParamName) => Some(Color::Red),
        (LuaTokenKind::TkName, _) => {
            let parent_kind = if let Some(p) = parent.parent() {
                match p.kind() {
                    LuaKind::Syntax(kind) => kind,
                    LuaKind::Token(_) => unreachable!(),
                }
            } else {
                return None;
            };

            match (node_kind, parent_kind) {
                (_, LuaSyntaxKind::CallExpr) => Some(Color::Blue),
                (_, LuaSyntaxKind::LocalFuncStat) => Some(Color::Blue),
                (LuaSyntaxKind::IndexExpr, LuaSyntaxKind::FuncStat) => Some(Color::Blue),
                _ => None,
            }
        }
        _ => None,
    }
}

pub struct LuaHighlighter;

impl reedline::Highlighter for LuaHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let tree = LuaParser::parse(line, ParserConfig::default());
        let root = tree.get_red_root();

        let mut text = StyledText::new();

        dbg!(&root);

        for token in root
            .descendants_with_tokens()
            .filter_map(|d| d.into_token())
        {
            let mut color = default_token_color(&token);

            if let Some(parent) = token.parent() {
                if let Some(new_color) = modify_token_color(&token, &parent) {
                    color = new_color;
                }
            }

            text.push((Style::new().fg(color), token.text().to_string()));
        }

        text
    }
}
