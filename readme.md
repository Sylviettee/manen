# Månen

Fancy Lua REPl! Featuring support for Lua 5.1-5.4, LuaJIT, and more!

## Features

* Syntax highlighting
* Syntax checking
* Formatted table outputs
* Saved REPL history
* Basic autocomplete

## Running

Månen has the following feature flags:
* `vendored` - Compile and embed Lua into the executable
* `lua51` - `lua54` - Use Lua 5.1-5.4 for the embedded runtime
* `luajit(52)` - Use LuaJIT(5.2 compatibility) for the embedded runtime

### Examples

```bash
cargo run # Uses vendored Lua 5.4 by default
cargo run --no-default-features lua52,vendored # Uses vendored Lua 5.2
cargo run --no-default-features lua53,vendored # Uses system Lua 5.3
```

## Additional runtimes

Månen can support any Lua runtime that has the following APIs
* `loadstring` / `load`
* `io.stdin`
* `io.write`
* `io.flush`

If you want state-preserving cancellation, `debug.sethook` is required.

## Configuration file

Configuration can be specified at `$XDG_CONFIG_HOME/manen/config.lua` or `$HOME/.config/manen/config.lua`.

```lua
-- default config.lua

-- embedded - Use the embedded Lua interpreter as specified in feature flags
-- system - Use a foreign runtime that meets the requirements in additional runtimes
--          If this option is specified, system_lua must be specified
manen.executor = 'embedded'

-- **full** path to Lua executable
manen.system_lua = nil

-- inspect - Use Lua-like table printing
-- address - Print addresses of tables like the original Lua REPL
-- comfytable - Use https://github.com/nukesor/comfy-table for table printing
manen.table_format = 'inspect'

-- size of history in terms of lines stored
manen.history_size = 256

-- if the output should be colored
manen.color_output = true
```
