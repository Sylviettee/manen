use std::{
    io::Write,
    process::Command,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
};

use mlua::prelude::*;
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use rexpect::session::{PtySession, spawn_command};
use send_wrapper::SendWrapper;
use tempfile::NamedTempFile;
use thiserror::Error;

use crate::inspect::format_string_bytes;

pub trait LuaExecutor: Send + Sync {
    fn exec(&self, code: &str) -> LuaResult<LuaValue>;
    fn globals(&self) -> LuaResult<LuaTable>;
    fn cancel(&self);
}

pub struct MluaExecutor {
    lua: Lua,
    cancelled: Arc<AtomicBool>,
}

impl MluaExecutor {
    pub fn new() -> Self {
        let lua = Lua::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        let inner_cancelled = cancelled.clone();
        lua.set_hook(LuaHookTriggers::EVERY_LINE, move |_lua, _debug| {
            if inner_cancelled.load(Ordering::Relaxed) {
                inner_cancelled.store(false, Ordering::Relaxed);

                return Err(LuaError::runtime("cancelled"));
            }

            Ok(LuaVmState::Continue)
        });

        Self { lua, cancelled }
    }
}

impl LuaExecutor for MluaExecutor {
    fn exec(&self, code: &str) -> LuaResult<LuaValue> {
        self.lua.load(code).set_name("=repl").eval()
    }

    fn globals(&self) -> LuaResult<LuaTable> {
        Ok(self.lua.globals())
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

pub struct SystemLuaExecutor {
    session: RwLock<SendWrapper<PtySession>>,
    program: String,
    lua: Lua,

    cancellation_file: RwLock<Option<NamedTempFile>>,
    pid: AtomicI32,
    is_stopping: AtomicBool,
}

#[derive(Debug, Error)]
pub enum SystemLuaError {
    #[error("lua error: {0}")]
    Lua(#[from] LuaError),
    #[error("expect error: {0}")]
    Expect(#[from] rexpect::error::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("restarted system Lua")]
    Restarted,
    #[error("runtime error")]
    RuntimeError(String),
}

enum RpcCommand {
    Globals,
    Exec(String),
    Prepare(String),
}

impl RpcCommand {
    pub fn to_lua(&self) -> String {
        match self {
            Self::Globals => String::from("globals"),
            Self::Exec(code) => format!("exec:{}", format_string_bytes(code.as_bytes(), false)),
            Self::Prepare(file) => format!("prepare:{file}"),
        }
    }
}

const RPC_CODE: &str = include_str!("../lua/rpc.lua");

impl SystemLuaExecutor {
    pub fn new(program: &str) -> Result<Self, SystemLuaError> {
        let (session, file) = Self::obtain_session(program)?;
        let pid = session.process.child_pid.as_raw();

        Ok(Self {
            session: RwLock::new(SendWrapper::new(session)),
            program: program.to_string(),
            lua: unsafe { Lua::unsafe_new() },
            cancellation_file: RwLock::new(file),
            pid: AtomicI32::new(pid),
            is_stopping: AtomicBool::new(false),
        })
    }

    fn obtain_session(
        program: &str,
    ) -> Result<(PtySession, Option<NamedTempFile>), SystemLuaError> {
        let mut cmd = Command::new(program);

        cmd.arg("-e");
        cmd.arg(RPC_CODE);

        let mut session = spawn_command(cmd, None)?;

        // TODO; should this be in our cache/run dir?
        let file = NamedTempFile::new()?;

        let prepare = RpcCommand::Prepare(file.path().to_string_lossy().to_string());

        let cmd = prepare.to_lua();
        session.send_line(&cmd)?;

        let lua = Lua::new();

        loop {
            let code = session.read_line()?;

            if let Ok(prepare_result) = lua.load(&code).eval::<LuaTable>() {
                if prepare_result.get::<bool>("data")? {
                    return Ok((session, Some(file)));
                } else {
                    return Ok((session, None));
                }
            }
        }
    }

    fn restart_process(&self, session: &mut SendWrapper<PtySession>) -> Result<(), SystemLuaError> {
        let (pty, file) = Self::obtain_session(&self.program)?;
        self.pid
            .store(pty.process.child_pid.as_raw(), Ordering::Relaxed);

        *session = SendWrapper::new(pty);

        let mut cancellation_file = self
            .cancellation_file
            .write()
            .expect("write cancellation_file");
        *cancellation_file = file;

        Ok(())
    }

    fn request(&self, command: RpcCommand) -> Result<LuaTable, SystemLuaError> {
        self.is_stopping.store(false, Ordering::Relaxed);

        let mut session = self.session.write().expect("write process");

        let cmd = command.to_lua();

        if session.send_line(&cmd).is_err() {
            // killed
            self.restart_process(&mut session)?;

            return Err(SystemLuaError::Restarted);
        }

        loop {
            let code = match session.read_line() {
                Ok(code) => code,
                Err(rexpect::error::Error::EOF { .. }) => {
                    self.restart_process(&mut session)?;

                    return Err(SystemLuaError::Restarted);
                }
                x => x?,
            };

            if let Ok(res) = self.lua.load(&code).eval::<LuaTable>() {
                if res.get::<String>("ty")? == "error" {
                    return Err(SystemLuaError::RuntimeError(res.get("data")?));
                };

                return Ok(res);
            } else {
                println!("{}", &code);
            }
        }
    }
}

impl LuaExecutor for SystemLuaExecutor {
    fn exec(&self, code: &str) -> LuaResult<LuaValue> {
        self.request(RpcCommand::Exec(code.to_string()))
            .map_err(LuaError::external)?
            .get("data")
    }

    fn globals(&self) -> LuaResult<LuaTable> {
        self.request(RpcCommand::Globals)
            .map_err(LuaError::external)?
            .get("data")
    }

    fn cancel(&self) {
        let mut cancellation_file = self
            .cancellation_file
            .write()
            .expect("write cancellation_file");

        if !self.is_stopping.load(Ordering::Relaxed) {
            self.is_stopping.store(true, Ordering::Relaxed);

            if let Some(file) = cancellation_file.as_mut() {
                if file.write_all(b"stop").is_ok() && file.flush().is_ok() {
                    return;
                }
            }
        }

        // Restart process
        let pid = self.pid.load(Ordering::Relaxed);
        let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
    }
}
