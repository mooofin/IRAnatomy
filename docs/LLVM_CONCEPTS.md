# LLVM Concepts (Practical Version)

This guide explains the LLVM ideas you see in this project, with focus on what is useful when reading outputs in the app.

## LLVM IR

LLVM IR (Intermediate Representation) is the compiler's working language between source code and machine code. It is lower-level than C/C++, but still readable. You can usually tell where values are created, where branches happen, and what operations are pure arithmetic versus memory access.

In this app, the first IR you see is generated from `clang++ -O0`. That view is useful as a baseline because it is closer to source structure and has not gone through the full optimization pipeline.

## Optimization levels (`-O0` to `-O3`)

Optimization levels choose how aggressively the compiler transforms code:

- `-O0` prioritizes debuggability and preserves structure.
- `-O1` applies a smaller set of cheap optimizations.
- `-O2` is the common default for performance-sensitive builds.
- `-O3` is more aggressive and may increase code size for speed.

In LLVM terms, each level maps to a pass pipeline. This app runs `opt -passes=default<O*>` so you can inspect the pipeline output instead of treating optimization as a black box.

## Passes

A pass is a single transformation or analysis over IR. Some passes simplify instructions, some propagate constants, some remove dead code, and some prepare IR for later code generation.

When `opt -print-after-all` is enabled, LLVM prints an IR snapshot after each pass. The app parses those snapshots into the timeline. As you move through passes, you are seeing the same function after repeated rewrites.

## SSA and values

Most LLVM IR is in SSA form (Static Single Assignment), which means each named value is assigned once. This makes data flow easier to analyze and enables many optimizations.

When reading IR in the app, look for:
- `%` local SSA values
- `@` globals/functions
- `phi` nodes at merge points

`phi` is especially important for loops and branches because it selects a value based on control-flow predecessor blocks.

## Basic blocks and CFG

A basic block is a straight-line sequence of instructions with one entry and one exit. A control flow graph (CFG) connects these blocks using edges for jumps and branches.

The CFG tab shows function-level graphs rendered from DOT output. Use it when you want to answer questions like:
- Did a branch disappear after simplification?
- Did loop structure change?
- Is an error path still reachable?

CFG is about possible paths, while IR text is about exact operations.

## IR diff view

The diff tab is not a source-level semantic proof. It is a visual aid to spot where blocks or instructions were added, removed, or rewritten between baseline and selected pass output.

Use it to quickly detect "where something changed", then confirm details in the raw IR tab.

## Assembly output

Assembly is the final low-level output for a target architecture. It reflects register allocation, calling convention details, instruction selection, and target-specific lowering decisions.

IR tells you compiler intent and transformations. Assembly tells you what will execute on hardware. Looking at both gives the best understanding.

## Common interpretation tips

- Start at `-O0` baseline IR to orient yourself.
- Step through passes and watch one function at a time.
- Use CFG when text IR feels too dense.
- Use assembly to validate whether an optimization likely impacts real machine instructions.

