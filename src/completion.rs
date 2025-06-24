use mlua::prelude::*;
use reedline::{Completer, Span, Suggestion};
use tree_sitter::{Parser, Point, Query, QueryCursor, Range, StreamingIterator, Tree};

#[derive(Debug)]
struct Variable {
    range: Range,
    name: String,
}

#[derive(Debug)]
struct Scope {
    range: Range,
    variables: Vec<Variable>,
}

pub struct LuaCompleter {
    lua: Lua,
    parser: Parser,
    tree: Tree,

    locals_query: Query,
    identifiers_query: Query,
    scopes: Vec<Scope>,
    text: String,
}

impl LuaCompleter {
    pub fn new(lua: Lua) -> Self {
        let mut parser = Parser::new();

        parser
            .set_language(&tree_sitter_lua::LANGUAGE.into())
            .unwrap();

        let tree = parser.parse("", None).unwrap();

        let locals_query = Query::new(
            &tree_sitter_lua::LANGUAGE.into(),
            include_str!("../queries/locals.scm"),
        )
        .unwrap();

        let identifiers_query = Query::new(
            &tree_sitter_lua::LANGUAGE.into(),
            "(identifier) @identifier",
        )
        .unwrap();

        Self {
            lua,
            parser,
            tree,
            locals_query,
            identifiers_query,
            scopes: Vec::new(),
            text: String::new(),
        }
    }

    fn refresh_tree(&mut self, text: &str) {
        self.tree = self.parser.parse(text, None).unwrap();
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
        let mut cursor = QueryCursor::new();

        let matches = cursor.matches(
            &self.locals_query,
            self.tree.root_node(),
            self.text.as_bytes(),
        );
        let names = self.locals_query.capture_names();

        let lines = self.text.split("\n").collect::<Vec<_>>();

        let mut scopes: Vec<Scope> = vec![
            // fallback scope
            Scope {
                range: Range {
                    start_byte: 0,
                    end_byte: self.text.len(),
                    start_point: Point::new(0, 0),
                    end_point: Point::new(lines.len(), lines.last().map_or("", |v| v).len()),
                },
                variables: Vec::new(),
            },
        ];
        let mut scope_hierarchy: Vec<usize> = vec![0];

        matches.for_each(|m| {
            for capture in m.captures {
                let name = names[capture.index as usize];
                let text = capture
                    .node
                    .utf8_text(self.text.as_bytes())
                    .unwrap()
                    .to_string();
                let range = capture.node.range();

                match name {
                    "local.definition" => {
                        let last = scope_hierarchy.last().unwrap();
                        let last_scope = &mut scopes[*last];

                        last_scope.variables.push(Variable { range, name: text });
                    }
                    "local.fn_name" => {
                        let len = scope_hierarchy.len();
                        let parent = scope_hierarchy
                            .get(len - 2)
                            .unwrap_or_else(|| scope_hierarchy.last().unwrap());
                        let scope = &mut scopes[*parent];

                        scope.variables.push(Variable { range, name: text });
                    }
                    "local.scope" => {
                        let scope = Scope {
                            range: capture.node.range(),
                            variables: Vec::new(),
                        };

                        if let Some(last) = scope_hierarchy.last() {
                            // outside
                            let last_scope = &scopes[*last];

                            if scope.range.end_byte > last_scope.range.end_byte {
                                scope_hierarchy.pop();
                            }
                        }

                        scope_hierarchy.push(scopes.len());
                        scopes.push(scope);
                    }
                    _ => {}
                }
            }
        });

        scopes
    }

    fn locals(&self, position: usize) -> Vec<String> {
        let mut variables = Vec::new();

        for scope in self.scopes.iter() {
            if position > scope.range.start_byte && position < scope.range.end_byte {
                for var in scope.variables.iter() {
                    if position > var.range.end_byte {
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
    fn autocomplete_upvalue(&self, query: &str, position: usize) -> Vec<String> {
        let mut upvalues = self.locals(position);
        upvalues.extend(self.globals());
        upvalues.sort();

        upvalues
            .into_iter()
            .filter(|s| s.starts_with(query))
            .collect()
    }

    fn current_identifier(&self, position: usize) -> Option<(Range, String)> {
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(
            &self.identifiers_query,
            self.tree.root_node(),
            self.text.as_bytes(),
        );

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let text = capture
                    .node
                    .utf8_text(self.text.as_bytes())
                    .unwrap()
                    .to_string();
                let range = capture.node.range();

                if position > range.start_byte && position < range.end_byte {
                    return Some((range, text.to_string()));
                }
            }
        }

        None
    }
}

impl Completer for LuaCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // TODO; proper autocomplete
        self.refresh_tree(line);

        if let Some((range, current)) = self.current_identifier(pos.saturating_sub(1)) {
            return self
                .autocomplete_upvalue(&current, pos)
                .into_iter()
                .map(|s| Suggestion {
                    value: s,
                    span: Span::new(range.start_byte, range.end_byte),
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

    fn line_to_position(line: usize, text: &str) -> usize {
        let split = text.split("\n").collect::<Vec<_>>();
        split[0..line].join("\n").len()
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
