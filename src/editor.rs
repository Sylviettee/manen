use std::{
    process,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use directories::ProjectDirs;
use mlua::prelude::*;
use reedline::{
    DefaultPrompt, DefaultPromptSegment, EditCommand, Emacs, FileBackedHistory, IdeMenu, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    default_emacs_keybindings,
};

use crate::{
    completion::LuaCompleter, format::TableFormat, hinter::LuaHinter, inspect::display_basic,
    parse::LuaHighlighter, validator::LuaValidator,
};

pub struct Editor {
    prompt: DefaultPrompt,
    editor: Reedline,
    lua: Lua,

    table_format: TableFormat,
    cancel_lua: Arc<AtomicBool>,
}

impl Editor {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let version: String = lua.globals().get("_VERSION")?;

        let cancel_lua = Arc::new(AtomicBool::new(false));

        let inner_cancel = cancel_lua.clone();
        lua.set_hook(LuaHookTriggers::EVERY_LINE, move |_lua, _debug| {
            if inner_cancel.load(Ordering::Relaxed) {
                inner_cancel.store(false, Ordering::Relaxed);

                return Err(LuaError::runtime("cancelled"));
            }

            Ok(LuaVmState::Continue)
        });

        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic(version),
            DefaultPromptSegment::Empty,
        );

        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu(String::from("completion_menu")),
                ReedlineEvent::MenuNext,
            ]),
        );
        keybindings.add_binding(
            KeyModifiers::ALT,
            KeyCode::Enter,
            ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
        );

        let ide_menu = IdeMenu::default().with_name("completion_menu");

        let mut editor = Reedline::create()
            .with_validator(Box::new(LuaValidator::new()))
            .with_completer(Box::new(LuaCompleter::new(lua.clone())))
            .with_highlighter(Box::new(LuaHighlighter))
            .with_hinter(Box::new(LuaHinter))
            .with_edit_mode(Box::new(Emacs::new(keybindings)))
            .with_menu(ReedlineMenu::EngineCompleter(Box::new(ide_menu)));

        if let Some(proj_dirs) = ProjectDirs::from("gay.gayest", "", "Manen") {
            let history = FileBackedHistory::with_file(256, proj_dirs.data_dir().join("history"));

            if let Ok(history) = history {
                editor = editor.with_history(Box::new(history))
            }
        }

        Ok(Self {
            prompt,
            editor,
            lua,

            table_format: TableFormat::ComfyTable(true),
            cancel_lua,
        })
    }

    fn register_ctrl_c(&self, is_running_lua: Arc<AtomicBool>) {
        let inner_cancel = self.cancel_lua.clone();

        ctrlc::set_handler(move || {
            if is_running_lua.load(Ordering::Relaxed) {
                inner_cancel.store(true, Ordering::Relaxed);
            } else {
                process::exit(0)
            }
        })
        .unwrap();
    }

    pub fn run(mut self) {
        let is_running_lua = Arc::new(AtomicBool::new(false));

        self.register_ctrl_c(is_running_lua.clone());

        loop {
            let signal = self.editor.read_line(&self.prompt);

            match signal {
                Ok(Signal::Success(line)) => {
                    if line.starts_with(".") {
                        if let Err(e) = self.eval_special(&line) {
                            eprintln!("{e}")
                        }

                        continue;
                    }

                    is_running_lua.store(true, Ordering::Relaxed);

                    if let Err(e) = self.eval(&line) {
                        eprintln!("{e}")
                    }

                    is_running_lua.store(false, Ordering::Relaxed);
                }
                //  TODO; this should cancel the current Lua execution if possible
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => break,
                _ => {}
            }
        }
    }

    // .help
    // .format <format> [true/false]
    fn eval_special(&mut self, line: &str) -> LuaResult<()> {
        let mut split = line.strip_prefix(".").unwrap().split_whitespace();

        let cmd = split.next();

        match cmd {
            Some("help") => {
                println!(".help\tPrint this message");
                println!(
                    ".format <inspect|address|comfytable> [true|false]\tConfigure table printing, boolean configures nesting"
                );
            }
            Some("format") => match split.next() {
                Some("inspect") => {
                    self.table_format = TableFormat::Inspect;
                }
                Some("address") => {
                    self.table_format = TableFormat::Address;
                }
                Some("comfytable") => {
                    let nested = split
                        .next()
                        .unwrap_or("")
                        .parse::<bool>()
                        .unwrap_or_default();

                    self.table_format = TableFormat::ComfyTable(nested);
                }
                _ => println!("unknown subcommand"),
            },
            _ => println!("unknown command"),
        }

        Ok(())
    }

    fn eval(&self, line: &str) -> LuaResult<()> {
        let value: LuaValue = self.lua.load(line).set_name("=stdin").eval()?;

        let stringify = match value {
            LuaValue::Table(tbl) => self.table_format.format(&tbl, true)?,
            value => display_basic(&value, true),
        };

        println!("{stringify}");

        Ok(())
    }
}
