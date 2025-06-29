use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use mlua::prelude::*;

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
