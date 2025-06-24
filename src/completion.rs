use std::collections::HashSet;

use mlua::prelude::*;
use tree_sitter::{Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

#[derive(Debug)]
struct Scope {
    start: Point,
    end: Point,
    variables: HashSet<String>,
}

pub struct LuaCompleter {
    lua: Lua,
    parser: Parser,
    tree: Tree,

    locals_query: Query,
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
            tree_sitter_lua::LOCALS_QUERY,
        )
        .unwrap();

        Self {
            lua,
            parser,
            tree,
            locals_query,
            text: String::new(),
        }
    }

    fn refresh_tree(&mut self, text: &str) {
        self.tree = self.parser.parse(text, None).unwrap();
        self.text = text.to_string();
    }

    fn globals(&self) -> Vec<LuaValue> {
        self.lua
            .globals()
            .pairs()
            .flatten()
            .map(|(k, _): (LuaValue, LuaValue)| k)
            .collect()
    }

    fn locals(&self, point: Point) -> Vec<String> {
        let mut cursor = QueryCursor::new();

        let matches = cursor.matches(
            &self.locals_query,
            self.tree.root_node(),
            self.text.as_bytes(),
        );
        let names = self.locals_query.capture_names();

        let mut scopes = Vec::new();
        let mut variables = Vec::new();

        matches.for_each(|m| {
            for capture in m.captures {
                let name = names[capture.index as usize];

                match name {
                    "local.definition" => {
                        variables.push((
                            capture.node.start_position(),
                            capture.node.utf8_text(self.text.as_bytes()).unwrap(),
                        ));
                    }
                    "local.scope" => {
                        scopes.push((capture.node.start_position(), capture.node.end_position()));
                    }
                    _ => {}
                }
            }
        });

        let new_scopes = scopes
            .into_iter()
            .map(|(start, end)| {
                let variables = variables
                    .iter()
                    .filter(|v| v.0 >= start && v.0 <= end)
                    .map(|v| v.1.to_string())
                    .collect::<HashSet<_>>();

                Scope {
                    start,
                    end,
                    variables,
                }
            })
            .collect::<Vec<_>>();

        dbg!(&new_scopes);

        Vec::new()
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
           print(a, b)
        end

        local function bar(c)
           print(c)
        end
        "#,
        );

        completer.locals(Point { row: 1, column: 0 });
    }
}
