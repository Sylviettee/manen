use mlua::prelude::*;
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

        Self {
            lua,
            parser,
            tree,
            locals_query,
            scopes: Vec::new(),
            text: String::new(),
        }
    }

    fn refresh_tree(&mut self, text: &str) {
        self.tree = self.parser.parse(text, None).unwrap();
        self.text = text.to_string();
        self.scopes = self.resolve_scopes();
    }

    fn globals(&self) -> Vec<LuaValue> {
        self.lua
            .globals()
            .pairs()
            .flatten()
            .map(|(k, _): (LuaValue, LuaValue)| k)
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

        let mut scopes: Vec<Scope> = Vec::new();
        let mut scope_hierarchy: Vec<usize> = Vec::new();

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

    fn locals(&self, point: Point) -> Vec<String> {
        let mut variables = Vec::new();

        for scope in self.scopes.iter() {
            if point > scope.range.start_point && point < scope.range.end_point {
                for var in scope.variables.iter() {
                    if point > var.range.end_point {
                        variables.push(var.name.clone());
                    }
                }
            }
        }

        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locals() {
        let mut completer = LuaCompleter::new(Lua::new());

        completer.refresh_tree(
            r#"
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
        "#,
        );

        assert_eq!(
            &["foo", "a", "b"].as_slice(),
            &completer.locals(Point { row: 2, column: 0 }),
        );

        assert_eq!(
            &["foo"].as_slice(),
            &completer.locals(Point { row: 6, column: 0 }),
        );

        assert_eq!(
            &["foo", "bar", "c"].as_slice(),
            &completer.locals(Point { row: 9, column: 0 }),
        );

        assert_eq!(
            &["foo", "bar"].as_slice(),
            &completer.locals(Point { row: 13, column: 0 }),
        );
    }
}
