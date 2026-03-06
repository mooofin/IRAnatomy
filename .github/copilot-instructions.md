# Copilot Instructions

## Project overview

LLVM IR Explorer is a full-stack Rust web app built with Leptos 0.6 and Axum. Users paste C/C++ source, select an optimization level, and explore the pass-by-pass LLVM IR transformation pipeline alongside assembly and CFG diagrams.

## Build and run

```bash
# Install dependencies (one-time)
sudo apt-get install -y clang llvm graphviz
cargo install cargo-leptos

# Dev server with hot reload (serves on http://localhost:3000)
cargo leptos watch

# Release build
cargo leptos build --release

# Docker
docker build -t llvm-ir-explorer .
docker run --rm -p 3000:3000 llvm-ir-explorer
```

There are no automated tests in this project. Manual testing is done by running `cargo leptos watch` and using the UI.

## Feature flags

The crate has three mutually exclusive Cargo features that control how the code compiles:

| Feature    | Purpose |
|------------|---------|
| `ssr`      | Server binary — Axum + Tokio, `leptos_axum`, UUID, tracing |
| `hydrate`  | WASM bundle — activates `hydrate()` entrypoint via `wasm-bindgen` |
| `csr`      | Client-side rendering only (no SSR) |

`cargo-leptos` selects features automatically: `ssr` for the binary, `hydrate` for the WASM lib. Feature-gating is done with `cfg_if!` blocks and `#[cfg(feature = "...")]` attributes throughout the code.

## Architecture

```
src/main.rs            — Axum server setup, route registration, static file serving
src/lib.rs             — Crate root; exposes modules; WASM hydrate() entrypoint
src/app.rs             — Root App component, routing, HomePage state management
src/components.rs      — CodeEditor, Timeline, OutputTabs UI components
src/server_functions.rs — CompileAndOptimize server function + data types
src/highlight.rs       — Custom IR/ASM syntax highlighter and LCS-based IR diff
style/main.css         — All styles
public/                — Static assets
```

### Request flow

1. User triggers compile → `compile_action` (a `create_server_action`) dispatches `CompileAndOptimize { code, opt_level }` to `/api/compile_and_optimize`.
2. Server function (`server_functions.rs`) creates a UUID-named temp dir, writes `input.cpp`, runs `clang++ -S -emit-llvm -O0` (baseline IR), `clang++ -S -O*` (assembly), `opt -passes=default<OX> -print-after-all` (pass snapshots), `opt -passes=dot-cfg` + `dot -Tsvg` (CFGs).
3. Returns `CompileResult { initial_ir, passes: Vec<OptimizationPass>, assembly, cfgs: Vec<Cfg> }`, then deletes the temp dir.
4. `create_effect` in `HomePage` unpacks the result into separate signals; a second effect maps `current_pass_index` → `optimized_ir`.

### Key data types (server_functions.rs)

```rust
pub struct CompileResult {
    pub initial_ir: String,
    pub passes: Vec<OptimizationPass>,  // pass name + IR snapshot
    pub assembly: String,
    pub cfgs: Vec<Cfg>,                 // function name + SVG string
}
```

## Key conventions

### Server functions
- Declared with `#[server(CompileAndOptimize, "/api")]`.
- **Must** be explicitly registered in `main.rs`: `server_fn::axum::register_explicit::<CompileAndOptimize>();`
- All server-only logic lives inside `#[cfg(feature = "ssr")]` blocks or the `ssr` feature-gated body of the server function.

### Leptos reactivity
- `create_rw_signal` for mutable state that needs both read and write access in one handle.
- `create_signal` returns a `(ReadSignal, WriteSignal)` pair; used for output values derived from server results.
- `create_memo` for derived/cached values (e.g., pre-highlighted HTML strings).
- `create_server_action` wraps a server function; check `.pending()` for loading state and `.value()` for the result.

### Syntax highlighting
- `highlight_ir` / `highlight_asm` in `highlight.rs` return HTML strings with `<span class='hl-*'>` tags.
- These are injected into `<pre>` elements using Leptos's `inner_html` prop — **never set raw user content with `inner_html`**, only the output of the highlighter (which escapes HTML first via `escape_html`).
- `diff_ir` uses an LCS DP algorithm capped at 500 lines per side; larger inputs fall back to plain highlighting without diff markup.

### Input safety
- Input size is hard-capped at **100,000 characters** in the server function — enforce this limit if adding new input paths.
- `clang++`, `opt`, and `dot` must be present in the runtime PATH; their absence causes a `ServerFnError::ServerError`.

### CSS class naming
- Component wrapper classes match component names: `code-editor-panel`, `output-panel`, `timeline-visualizer`.
- Highlighting span classes: `hl-comment`, `hl-label`, `hl-keyword`, `hl-type`, `hl-value`, `hl-global`, `hl-string`, `hl-number`, `hl-meta`, `hl-attr`.
- Diff spans: `diff-add`, `diff-remove`.
