# Architecture

LLVM IR Explorer is split into two clear parts: a Leptos UI and a Rust server pipeline that shells out to LLVM tools. The UI is responsible for input, navigation, and rendering. The server is responsible for compilation, analysis, and shaping outputs into a response the UI can consume.

![Architecture flow](./architecture-flow.svg)

## End-to-end request flow

When a user clicks compile, the UI dispatches the `CompileAndOptimize` server action with two fields: source code and optimization level. The server function in `src/server_functions.rs` receives this payload, validates it, and creates a temporary working directory.

Inside that temp directory, the backend runs a deterministic sequence of commands. It first generates baseline LLVM IR (`clang++ -S -emit-llvm -O0`). Then it runs `opt` with `-print-after-all` to capture pass-by-pass IR states for the selected optimization pipeline. It separately generates assembly with `clang++ -S -O*`. Finally, it generates CFG files with `opt -passes=dot-cfg` and converts them to SVG with Graphviz `dot`.

The server converts those artifacts into a `CompileResult` payload with `initial_ir`, `passes`, `assembly`, and `cfgs`. That payload is returned to the client and temp files are removed.

## Frontend responsibilities

The UI composition is centered in `src/app.rs` and `src/components.rs`. `CodeEditor` captures source and compile options. `Timeline` lets users move through optimization passes. `OutputTabs` renders LLVM IR, optimized IR, a side-by-side diff, assembly, and CFGs. Highlighting and diff markup are done in `src/highlight.rs`.

State updates are intentionally reactive. A successful compile response updates all views in one place, and moving the pass slider updates the optimized IR tab by selecting the corresponding pass snapshot.

## Server responsibilities

Routing and SSR bootstrapping are in `src/main.rs`. The API endpoint is exposed under `/api/*fn_name`, and the `CompileAndOptimize` server function is explicitly registered. The server function is the trust boundary where untrusted user code enters the system, so this is the place to enforce limits, sanitize behavior, and shape failure responses.

## Operational notes

The runtime environment must include `clang++`, `opt`, and `dot` in PATH. If any one of these binaries is missing, the compile request will fail. The project currently applies an input size guard of 100,000 characters to reduce abuse risk and prevent oversized compilation jobs.

