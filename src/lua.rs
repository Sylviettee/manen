use std::{io::{self, Write}, process::{Child, ChildStdin, ChildStdout, Command, Stdio}, sync::{
    atomic::{AtomicBool, Ordering}, Arc, RwLock
}};

use mlua::prelude::*;

use crate::inspect::format_string_bytes;

pub trait LuaExecutor: Send + Sync {
    fn exec(&self, code: &str) -> LuaResult<LuaValue>;
    fn globals(&self) -> LuaTable;
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

    fn globals(&self) -> LuaTable {
        self.lua.globals()
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

const LUA_RPC: &str = include_str!("../lua/rpc.lua");

pub struct SystemLuaExecutor {
    child: RwLock<Child>,
    stdin: RwLock<ChildStdin>,
    stdout: RwLock<ChildStdout>,

    program: String,
}

pub enum RpcCommand {
    Globals,
    Ping,
    Exec(String),
}

impl RpcCommand {
    pub fn to_lua(&self) -> String {
        let func = match self {
            Self::Globals => "globals",
            Self::Ping => "ping",
            Self::Exec(_) => "exec"
        };

        let param = if let Self::Exec(code) = self {
            format_string_bytes(code.as_bytes(), false)
        } else {
            String::new()
        };

        format!("rpc.{func}({param})")
    }
}

impl SystemLuaExecutor {
    pub fn new(program: &str) -> io::Result<Self> {
        let (child, stdin, stdout) = Self::obtain_process(program)?;

        Ok(Self {
            child: RwLock::new(child),
            stdin: RwLock::new(stdin),
            stdout: RwLock::new(stdout),

            program: program.to_string(),
        })
    }

    fn obtain_process(program: &str) -> io::Result<(Child, ChildStdin, ChildStdout)> {
        let mut child = Command::new(program)
            .stderr(Stdio::null())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();

        stdin.write_all(LUA_RPC.as_bytes())?;

        let stdout = child.stdout.take().unwrap();

        Ok((child, stdin, stdout))
    }
}

impl LuaExecutor for SystemLuaExecutor {
    fn exec(&self, code: &str) -> LuaResult<LuaValue> {
        todo!()
    }

    fn globals(&self) -> LuaTable {
        todo!()
    }

    fn cancel(&self) {
        todo!()
    }
}
