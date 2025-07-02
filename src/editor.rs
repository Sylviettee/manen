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
    completion::LuaCompleter, config::Config, hinter::LuaHinter, inspect::display_basic,
    lua::LuaExecutor, parse::LuaHighlighter, validator::LuaValidator,
};

pub struct Editor {
    prompt: DefaultPrompt,
    editor: Reedline,
    lua_executor: Arc<dyn LuaExecutor>,
    config: Config,
}

impl Editor {
    pub fn new() -> LuaResult<Self> {
        let config = Config::load()?;
        let lua_executor = config.get_executor().map_err(LuaError::external)?;

        let version: String = lua_executor.globals()?.get("_VERSION")?;

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
            .with_completer(Box::new(LuaCompleter::new(
                lua_executor.clone() as Arc<dyn LuaExecutor>
            )))
            .with_highlighter(Box::new(LuaHighlighter))
            .with_hinter(Box::new(LuaHinter))
            .with_edit_mode(Box::new(Emacs::new(keybindings)))
            .with_menu(ReedlineMenu::EngineCompleter(Box::new(ide_menu)));

        if let Some(proj_dirs) = ProjectDirs::from("gay.gayest", "", "Manen") {
            let history = FileBackedHistory::with_file(
                config.history_size,
                proj_dirs.data_dir().join("history"),
            );

            if let Ok(history) = history {
                editor = editor.with_history(Box::new(history))
            }
        }

        Ok(Self {
            prompt,
            editor,
            lua_executor,
            config,
        })
    }

    fn register_ctrl_c(&self, is_running_lua: Arc<AtomicBool>) {
        let executor = self.lua_executor.clone();

        ctrlc::set_handler(move || {
            if is_running_lua.load(Ordering::Relaxed) {
                executor.cancel();
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
                    is_running_lua.store(true, Ordering::Relaxed);

                    if let Err(e) = self.eval(&line) {
                        eprintln!("{e}")
                    }

                    is_running_lua.store(false, Ordering::Relaxed);
                }
                Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => break,
                _ => {}
            }
        }
    }

    fn eval(&self, line: &str) -> LuaResult<()> {
        let value: LuaValue = self.lua_executor.exec(line)?;
        let config = &self.config;

        let stringify = match value {
            LuaValue::Table(tbl) => config.table_format.format(&tbl, config.color_output)?,
            value => display_basic(&value, config.color_output),
        };

        println!("{stringify}");

        Ok(())
    }
}
