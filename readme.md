# Månen

Fancy Lua REPl! Featuring support for Lua 5.1-5.4, LuaJIT, and Luau!

## Features

* Syntax highlighting
* Syntax checking
* Formatted table outputs
* Saved REPL history (TODO)

## Running

```bash
cargo run # Uses vendored Lua 5.4 by default
cargo run --no-default-features --features lua52 # Uses system Lua 5.2
cargo run --no-default-features --features vendored,luau[-vector4] # Uses vendored Luau (with vector4)
cargo run --no-default-features --features vendored,luau-jit # Uses vendored Luau with JIT
```
