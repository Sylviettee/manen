use emmylua_parser::{
    LuaAst, LuaAstNode, LuaAstToken, LuaBlock, LuaClosureExpr, LuaNameExpr, LuaParser,
    LuaSyntaxTree, ParserConfig,
};
use mlua::prelude::*;
use reedline::{Completer, Span, Suggestion};
use rowan::TextRange;

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
    lua: Lua,
    tree: LuaSyntaxTree,

    scopes: Vec<Scope>,
    text: String,
}

impl LuaCompleter {
    pub fn new(lua: Lua) -> Self {
        Self {
            lua,
            tree: LuaParser::parse("", ParserConfig::default()),
            scopes: Vec::new(),
            text: String::new(),
        }
    }

    fn refresh_tree(&mut self, text: &str) {
        self.tree = LuaParser::parse(text, ParserConfig::default());
        self.text = text.to_string();
        self.scopes = self.resolve_scopes();
    }

    fn globals(&self) -> Vec<String> {
        self.lua
            .globals()
            .pairs()
            .flatten()
            .map(|(k, _): (String, LuaValue)| k)
            .collect()
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

            // TODO; for loops

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

        dbg!(&scopes);

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

    fn current_identifier(&self, position: u32) -> Option<(TextRange, String)> {
        None
    }
}

impl Completer for LuaCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let pos = pos as u32;
        // TODO; proper autocomplete
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

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_to_position(line: usize, text: &str) -> u32 {
        let split = text.split("\n").collect::<Vec<_>>();
        split[0..line].join("\n").len() as u32
    }

    #[test]
    fn locals() {
        let mut completer = LuaCompleter::new(Lua::new());

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
        let lua = Lua::new();
        lua.globals().set("foobar", "").unwrap();

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
}
