# LLVM IR Explorer

LLVM IR Explorer is a web app for understanding what the compiler is actually doing to your code. You paste C/C++, choose an optimization level, and inspect the full path from baseline LLVM IR to optimized IR, assembly, and control flow graphs.

If you are learning LLVM or debugging surprising performance behavior, the key idea is that most interesting changes happen in IR, not directly in assembly. IR is structured enough to read and reason about, while still being low-level enough to show real compiler decisions. This project is built to make those decisions visible pass by pass.

The app shows initial IR from `-O0`, optimized snapshots from `opt -print-after-all`, assembly for the selected level, and CFG diagrams rendered as SVG. A CFG is a graph of basic blocks and branch edges, and it helps when you want to understand how loops and conditionals were simplified over time.

## Documentation map

- [Architecture diagram and flow](./docs/ARCHITECTURE.md)
- [LLVM concepts guide](./docs/LLVM_CONCEPTS.md)

## Architecture at a glance

![Architecture flow](./docs/architecture-flow.svg)

The UI (Leptos) sends source and optimization level to a server function. The backend runs `clang++`, `opt`, and `dot` in a temporary workspace, then returns structured results containing IR snapshots, assembly, and CFG SVGs. The UI renders those results with syntax highlighting, pass timeline navigation, and diff views.

## LLVM concepts (quick intro)

LLVM IR is an intermediate language between source code and machine code. It is where most compiler optimizations run. By comparing baseline IR (`-O0`) with pass-by-pass snapshots from `opt`, you can see exactly where values are folded, branches are simplified, and dead code is removed.

Optimization levels (`-O0` to `-O3`) are not single switches, but predefined pass pipelines. This app exposes those pipeline stages through a timeline. Instead of one opaque "optimized output", you can inspect each transition and understand which pass likely caused a transformation.

CFGs complement raw IR text. IR shows operations; CFG shows possible execution paths across basic blocks. When branch-heavy code changes shape, the CFG tab makes that easier to see quickly.

For the fuller explanation, examples, and reading tips, see [LLVM concepts guide](./docs/LLVM_CONCEPTS.md).

## Local development

Install Rust, `cargo-leptos`, LLVM/Clang, and Graphviz. On Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y clang llvm graphviz
cargo install cargo-leptos
```

Run in dev mode from repo root:

```bash
cargo leptos watch
```

Open `http://localhost:3000`.

Build release assets:

```bash
cargo leptos build --release
```

If you prefer Docker:

```bash
docker build -t llvm-ir-explorer .
docker run --rm -p 3000:3000 llvm-ir-explorer
```

And open `http://localhost:3000`.

## Runtime notes

This app expects `clang++`, `opt`, and `dot` in server runtime PATH. Compile input is currently capped at 100,000 characters.
