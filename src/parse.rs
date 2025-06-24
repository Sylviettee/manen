use emmylua_parser::{LuaAst, LuaAstNode, LuaSyntaxTree};
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
        LuaAst::LuaForStat(_) => Some("for"),
        LuaAst::LuaForRangeStat(_) => Some("for_range"),
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
        LuaAst::LuaCallExprStat(_)
            | LuaAst::LuaLabelStat(_)
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
