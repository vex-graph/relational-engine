# C search

The production direction is C search over Rust-owned storage at R2, alongside
Vexspoke. `primitives/` owns primitive-span search. Filesystem gathering/indexing
belongs to Rust `io/`; this is not a second file watcher or GPU driver.

This directory currently documents the boundary only, with no callable search
API. The imported C references elsewhere in `src/` are preserved unchanged.
