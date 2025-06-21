use mlua::prelude::*;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

pub enum TableFormat {
    Ascii,
    Lua,
    Address,
}

pub struct Editor {
    prompt: DefaultPrompt,
    editor: Reedline,
    lua: Lua,

    table_format: TableFormat,
    print_nested_tables: bool,
}

impl Editor {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let version: String = lua.globals().get("_VERSION")?;

        let editor = Reedline::create();
        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic(version),
            DefaultPromptSegment::Empty,
        );

        Ok(Self {
            prompt,
            editor,
            lua,

            table_format: TableFormat::Address,
            print_nested_tables: false,
        })
    }

    pub fn run(mut self) {
        loop {
            let signal = self.editor.read_line(&self.prompt);

            match signal {
                Ok(Signal::Success(line)) => {
                    if let Err(e) = self.eval(&line) {
                        eprintln!("{e}")
                    }
                }
                //  TODO; this should cancel the current Lua execution if possible
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                    println!("aborted");
                    break
                },
                _ => {}
            }
        }
    }

    fn eval(&mut self, line: &str) -> LuaResult<()> {
        let res: LuaValue = self.lua.load(line).eval()?;

        // TODO; pretty print tables
        println!("{}", res.to_string()?);

        Ok(())
    }
}
