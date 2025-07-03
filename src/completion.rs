use std::sync::Arc;

use emmylua_parser::{
    LuaAst, LuaAstNode, LuaAstToken, LuaBlock, LuaExpr, LuaIndexExpr, LuaNameExpr, LuaParser,
    LuaSyntaxTree, LuaTokenKind,
};
use mlua::prelude::*;
use reedline::{Completer, Span, Suggestion};
use rowan::{TextRange, TextSize};

use crate::{lua::LuaExecutor, parse};

#[derive(Debug)]
struct Variable {
    range: TextRange,
    name: String,
}

#[derive(Debug)]
struct Scope {
    range: TextRange,
    variables: Vec<Variable>,
}

pub struct LuaCompleter {
    lua_executor: Arc<dyn LuaExecutor>,
    tree: LuaSyntaxTree,

    scopes: Vec<Scope>,
    text: String,
}

impl LuaCompleter {
    pub fn new(lua_executor: Arc<dyn LuaExecutor>) -> Self {
        Self {
            lua_executor,
            tree: LuaParser::parse("", parse::config()),
            scopes: Vec::new(),
            text: String::new(),
        }
    }

    fn refresh_tree(&mut self, text: &str) {
        self.tree = LuaParser::parse(text, parse::config());
        self.text = text.to_string();
        self.scopes = self.resolve_scopes();
    }

    fn globals(&self) -> Vec<String> {
        if let Ok(globals) = self.lua_executor.globals() {
            globals
                .pairs()
                .flatten()
                .map(|(k, _): (String, LuaValue)| k)
                .collect()
        } else {
            Vec::new()
        }
    }

    fn resolve_scopes(&self) -> Vec<Scope> {
        let mut scopes = Vec::new();

        let chunk = self.tree.get_chunk_node();

        for scope in chunk.descendants::<LuaBlock>() {
            let mut variables = Vec::new();

            match scope.get_parent() {
                Some(LuaAst::LuaClosureExpr(closure)) => {
                    if let Some(params) = closure.get_params_list() {
                        for param in params.get_params() {
                            if let Some(token) = param.get_name_token() {
                                variables.push(Variable {
                                    range: param.get_range(),
                                    name: token.get_name_text().to_string(),
                                });
                            }
                        }
                    }
                }
                Some(LuaAst::LuaForRangeStat(range)) => {
                    for token in range.get_var_name_list() {
                        variables.push(Variable {
                            range: token.get_range(),
                            name: token.get_name_text().to_string(),
                        })
                    }
                }
                Some(LuaAst::LuaForStat(stat)) => {
                    if let Some(token) = stat.get_var_name() {
                        variables.push(Variable {
                            range: token.get_range(),
                            name: token.get_name_text().to_string(),
                        });
                    }
                }
                _ => {}
            }

            for node in scope.children::<LuaAst>() {
                match node {
                    LuaAst::LuaLocalFuncStat(stat) => {
                        if let Some(name) = stat.get_local_name() {
                            if let Some(token) = name.get_name_token() {
                                variables.push(Variable {
                                    range: token.get_range(),
                                    name: token.get_name_text().to_string(),
                                });
                            }
                        }
                    }
                    LuaAst::LuaLocalStat(stat) => {
                        for name in stat.get_local_name_list() {
                            if let Some(token) = name.get_name_token() {
                                variables.push(Variable {
                                    range: stat.get_range(),
                                    name: token.get_name_text().to_string(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }

            scopes.push(Scope {
                range: scope.get_range(),
                variables,
            });
        }

        scopes
    }

    fn locals(&self, position: u32) -> Vec<String> {
        let mut variables = Vec::new();

        for scope in self.scopes.iter() {
            if position >= scope.range.start().into() && position <= scope.range.end().into() {
                for var in scope.variables.iter() {
                    if position >= var.range.end().into() {
                        variables.push(var.name.clone());
                    }
                }
            }
        }

        variables
    }

    // okay not the correct terminology
    //
    // there are 3 kinds of variable
    // - local (current scope)
    // - global (_G/_ENV)
    // - upvalue (local of parent scope(s))
    //
    // well in 5.2+ its only local and upvalue since you upvalue _ENV
    // then you get the individual global variable
    //
    // in the code
    //
    // ```lua
    // local a = 1
    // b = 2
    //
    // local function _()
    //    local c = 3
    //    print(a, b, c)
    // end
    // ```
    //
    // the bytecode for the function is
    //
    // 1       [5]     LOADI           0 3
    // 2       [6]     GETTABUP        1 0 0   ; _ENV "print"
    // 3       [6]     GETUPVAL        2 1     ; a
    // 4       [6]     GETTABUP        3 0 1   ; _ENV "b"
    //
    // the local can be loaded with LOADI (load integer) while a and b
    // both have to be upvalued
    //
    // this is different in 5.1
    //
    // 1       [5]     LOADK           0 -1    ; 3
    // 2       [6]     GETGLOBAL       1 -2    ; print
    // 3       [6]     GETUPVAL        2 0     ; a
    // 4       [6]     GETGLOBAL       3 -3    ; b
    //
    // in 5.1, globals are treated uniquely and given their own opcode
    //
    // to summarize, this function is not properly named
    //
    // globals either exist or are an extension of _ENV
    fn autocomplete_upvalue(&self, query: &str, position: u32) -> Vec<String> {
        let mut upvalues = self.locals(position);
        upvalues.extend(self.globals());
        upvalues.sort();

        upvalues
            .into_iter()
            .filter(|s| s.starts_with(query))
            .collect()
    }

    fn table_index(&self, position: u32) -> Option<(TextRange, Vec<String>)> {
        let chunk = self.tree.get_chunk_node();

        for index in chunk.descendants::<LuaIndexExpr>() {
            let (range, name, is_dot) = index
                .get_index_key()
                .map(|k| k.get_range().map(|r| (r, k.get_path_part(), false)))
                .unwrap_or_else(|| {
                    index.token_by_kind(LuaTokenKind::TkDot).map(|t| {
                        let range = t.get_range();
                        (
                            TextRange::new(range.start(), range.start() + TextSize::new(1)),
                            String::new(),
                            true,
                        )
                    })
                })?;

            if position >= range.start().into() && position < range.end().into() {
                let mut children: Vec<String> = Vec::new();

                for parent_index in index.descendants::<LuaIndexExpr>() {
                    if let Some(token) = parent_index.get_name_token() {
                        children.push(token.get_name_text().to_string());
                    }

                    if let Some(LuaExpr::NameExpr(token)) = parent_index.get_prefix_expr() {
                        children.push(token.get_name_text()?);
                    }
                }

                if children.len() > 1 {
                    children.reverse();
                    children.pop();
                }

                let fields = if let Ok(globals) = self.lua_executor.globals() {
                    let mut var: LuaResult<LuaValue> = Ok(LuaValue::Table(globals));

                    for index in children.iter().rev() {
                        if let Ok(LuaValue::Table(tbl)) = var {
                            var = tbl.raw_get(index.as_str())
                        }
                    }

                    if let Ok(LuaValue::Table(tbl)) = var {
                        tbl.pairs()
                            .flatten()
                            .map(|(k, _): (String, LuaValue)| k)
                            .filter(|s| s.starts_with(&name))
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                if is_dot {
                    return Some((
                        TextRange::new(range.start() + TextSize::new(1), range.end()),
                        fields,
                    ));
                } else {
                    return Some((range, fields));
                }
            }
        }

        None
    }

    fn current_identifier(&self, position: u32) -> Option<(TextRange, String)> {
        let chunk = self.tree.get_chunk_node();

        for identifier in chunk.descendants::<LuaNameExpr>() {
            let range = identifier.get_range();

            if position >= range.start().into() && position < range.end().into() {
                if let Some(name) = identifier.get_name_text() {
                    return Some((range, name));
                } else {
                    return None;
                }
            }
        }

        None
    }
}

impl Completer for LuaCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let pos = pos as u32;
        self.refresh_tree(line);

        if let Some((range, current)) = self.current_identifier(pos.saturating_sub(1)) {
            return self
                .autocomplete_upvalue(&current, pos)
                .into_iter()
                .map(|s| Suggestion {
                    value: s,
                    span: Span::new(range.start().into(), range.end().into()),
                    ..Default::default()
                })
                .collect();
        }

        if let Some((range, fields)) = self.table_index(pos.saturating_sub(1)) {
            return fields
                .into_iter()
                .map(|s| Suggestion {
                    value: s,
                    span: Span::new(range.start().into(), range.end().into()),
                    ..Default::default()
                })
                .collect();
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::lua::MluaExecutor;

    use super::*;

    fn lua_executor() -> Arc<dyn LuaExecutor> {
        Arc::new(MluaExecutor::new())
    }

    fn line_to_position(line: usize, text: &str) -> u32 {
        let split = text.split("\n").collect::<Vec<_>>();
        split[0..line].join("\n").len() as u32
    }

    #[test]
    fn locals() {
        let mut completer = LuaCompleter::new(lua_executor());

        let text = r#"
        local function foo(a, b)
           -- 2: foo, a, b
           print(a, b)
        end

        -- 6: foo

        local function bar(c)
           -- 9: foo, bar, c
           print(c)
        end

        -- 13: foo, bar

        for i = 1, 10 do
           -- 16: foo, bar, i
           print(i)
        end

        -- 20: foo, bar

        for i, v in pairs(_G) do
           -- 23: foo, bar, i, v
           print(i, v)
        end

        -- 27: foo, bar
        "#;

        completer.refresh_tree(text);

        assert_eq!(
            &["foo", "a", "b"].as_slice(),
            &completer.locals(line_to_position(2, text)),
        );

        assert_eq!(
            &["foo"].as_slice(),
            &completer.locals(line_to_position(6, text)),
        );

        assert_eq!(
            &["foo", "bar", "c"].as_slice(),
            &completer.locals(line_to_position(9, text)),
        );

        assert_eq!(
            &["foo", "bar"].as_slice(),
            &completer.locals(line_to_position(13, text)),
        );

        assert_eq!(
            &["foo", "bar", "i"].as_slice(),
            &completer.locals(line_to_position(16, text)),
        );

        assert_eq!(
            &["foo", "bar"].as_slice(),
            &completer.locals(line_to_position(20, text)),
        );

        assert_eq!(
            &["foo", "bar", "i", "v"].as_slice(),
            &completer.locals(line_to_position(23, text)),
        );

        assert_eq!(
            &["foo", "bar"].as_slice(),
            &completer.locals(line_to_position(27, text)),
        );
    }

    #[test]
    fn upvalues() {
        let lua = lua_executor();
        lua.globals().unwrap().set("foobar", "").unwrap();

        let mut completer = LuaCompleter::new(lua);

        let text = r#"
        local function foo(a, fooing)
            local foobaz = 3
            -- 3: foo, foobar, fooing, foobaz
        end
        "#;

        completer.refresh_tree(text);

        assert_eq!(
            &["foo", "foobar", "foobaz", "fooing"]
                .map(|s| s.to_string())
                .as_slice(),
            &completer.autocomplete_upvalue("foo", line_to_position(3, text))
        );
    }

    #[test]
    fn table_index_query() {
        let lua = lua_executor();

        let mut completer = LuaCompleter::new(lua);

        completer.refresh_tree("print(table.ins");

        assert_eq!(
            &["insert"].map(|s| s.to_string()).as_slice(),
            &completer.table_index(14).map(|t| t.1).unwrap()
        );
    }

    #[test]
    fn table_index_all() {
        let lua = lua_executor();

        lua.globals()
            .unwrap()
            .set("foo", HashMap::from([("bar", 1), ("baz", 2), ("ipsum", 3)]))
            .unwrap();

        let mut completer = LuaCompleter::new(lua);

        completer.refresh_tree("print(foo.");

        let mut fields = completer.table_index(9).map(|t| t.1).unwrap();
        fields.sort();

        assert_eq!(
            &["bar", "baz", "ipsum"].map(|s| s.to_string()).as_slice(),
            &fields
        );
    }
}
