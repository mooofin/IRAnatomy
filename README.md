# LLVM IR Explorer

Paste C or C++, pick an optimization level, and walk through what the compiler actually did to your code, pass by pass. You get the initial IR at `-O0`, a snapshot after every optimization pass, assembly for the selected level, and CFG diagrams for each function. IR and assembly can be downloaded straight from the output tabs.

Most tools give you a before and after. This one shows you everything in between.

## What each tab shows

**LLVM IR** shows the baseline IR compiled at `-O0` before any optimizations run. This is the raw translation of your source into SSA form and is useful as a starting point because it still resembles the source structure.

**Optimized IR** shows the IR state after whichever pass is selected in the timeline. Use the slider or click a pass name to jump around.

**IR Diff** puts the initial IR and the current pass side by side. Changed lines are highlighted. The diff uses LCS so small edits do not flood the view. It is a visual aid for spotting where blocks or instructions were added, removed, or rewritten, not a semantic proof.

**Assembly** shows the final output from `clang++` at the selected optimization level. IR tells you compiler intent, assembly tells you what will actually run on hardware.

**CFG** renders a control flow graph per function as SVG. Use it when you want to see whether a branch disappeared after simplification, whether loop structure changed, or whether an error path is still reachable.

## Architecture

![Architecture diagram](./docs/architecture-flow.svg)

The frontend is a Leptos app that compiles to both a server binary (SSR with Axum) and a WASM bundle (hydration). When you click COMPILE, it dispatches a server action to `/api/compile_and_optimize`. The server creates a UUID-named temp directory, writes your source to `input.cpp`, then runs:

1. `clang++ -S -emit-llvm -O0` to get baseline IR
2. `opt -passes=default<O*> -print-after-all` to capture every pass snapshot
3. `clang++ -S -O*` for assembly
4. `opt -passes=dot-cfg` then `dot -Tsvg` for control flow graphs

Everything is bundled into a single response and the temp directory is deleted before it returns.

On the frontend, a `create_effect` unpacks the result into separate signals. A second effect maps `current_pass_index` to the right IR snapshot so the output updates as you move through the timeline.

## LLVM background

LLVM IR is the compiler's working language between source code and machine code. It is lower-level than C/C++ but still readable. You can usually tell where values are created, where branches happen, and what operations are pure arithmetic versus memory access.

Most LLVM IR is in SSA form (Static Single Assignment), which means each named value is assigned once. When reading IR in the app, look for `%` local values, `@` globals and functions, and `phi` nodes at control flow merge points. `phi` is important for loops and branches because it selects a value based on which predecessor block was taken.

Optimization levels are not single switches, they are predefined pass pipelines. This app runs `opt -passes=default<O*>` and exposes the pipeline stage by stage so you can see exactly where constants were folded, branches simplified, or dead code removed, rather than treating it as a black box.

A basic block is a straight-line sequence of instructions with one entry and one exit. The CFG connects these blocks with edges for jumps and branches. Use the CFG tab when text IR feels too dense and you want to reason about possible execution paths instead of exact operations.

Useful starting point: orient at `-O0`, step through passes watching one function, switch to CFG when branching is the question, then check assembly to confirm whether the optimization actually changes real machine instructions.

## Running it locally

### With Nix (recommended)

Use the flake-based development shell:

```bash
nix develop
```

Then run the app:

```bash
cargo leptos watch
```

Open `http://localhost:3000`.

If `cargo-leptos` is not available from your current nixpkgs revision, install it once inside the shell:

```bash
cargo install cargo-leptos
```

### Without Nix (Debian/Ubuntu)

You need Rust, `cargo-leptos`, clang, LLVM, and Graphviz. On Debian/Ubuntu:

```bash
sudo apt-get install -y clang llvm graphviz
cargo install cargo-leptos
```

Start the dev server with hot reload:

```bash
cargo leptos watch
```

Open `http://localhost:3000`.

Build for release:

```bash
cargo leptos build --release
./target/release/llvm-ir-explorer
```

Or via Docker:

```bash
docker build -t llvm-ir-explorer .
docker run --rm -p 3000:3000 llvm-ir-explorer
```

## Notes

`clang++`, `opt`, and `dot` must be in PATH at runtime. If any are missing, compile requests will fail with an error shown in the UI.

Input is capped at 100,000 characters.
