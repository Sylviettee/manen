use std::{
    process::Command,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use mlua::prelude::*;
use rexpect::session::{PtySession, spawn_command};
use send_wrapper::SendWrapper;
use thiserror::Error;

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
    process: RwLock<SendWrapper<PtySession>>,
    program: String,
    lua: Lua,
}

#[derive(Debug, Error)]
pub enum SystemLuaError {
    #[error("lua error: {0}")]
    Lua(#[from] LuaError),
    #[error("expect error: {0}")]
    Expect(#[from] rexpect::error::Error),
    #[error("restarted system Lua")]
    Restarted,
    #[error("runtime error")]
    RuntimeError(String),
}

enum RpcCommand {
    Globals,
    Exec(String),
}

impl RpcCommand {
    pub fn to_lua(&self) -> String {
        let func = match self {
            Self::Globals => "globals",
            Self::Exec(_) => "exec",
        };

        if let Self::Exec(code) = self {
            format!("{func}:{code}")
        } else {
            func.to_string()
        }
    }
}

const RPC_CODE: &str = include_str!("../lua/rpc.lua");

impl SystemLuaExecutor {
    pub fn new(program: &str) -> Result<Self, SystemLuaError> {
        Ok(Self {
            process: RwLock::new(SendWrapper::new(Self::obtain_process(program)?)),
            program: program.to_string(),
            lua: unsafe { Lua::unsafe_new() },
        })
    }

    fn obtain_process(program: &str) -> Result<PtySession, SystemLuaError> {
        let mut cmd = Command::new(program);

        cmd.arg("-e");
        cmd.arg(RPC_CODE);

        Ok(spawn_command(cmd, None)?)
    }

    fn request(&self, command: RpcCommand) -> Result<LuaTable, SystemLuaError> {
        let mut process = self.process.write().expect("write process");

        let cmd = command.to_lua();
        process.send_line(&cmd)?;

        loop {
            let code = match process.read_line() {
                Ok(code) => code,
                Err(rexpect::error::Error::EOF { .. }) => {
                    *process = SendWrapper::new(Self::obtain_process(&self.program)?);
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
        todo!()
    }
}
