# relational-engine
memory engine that self describes and has coexistence

## Starting point: two interpretations

- `src/`: preserved C23 reference/scratchpad, imported from Vexspoke.
  `io/`, `relational/`, and `reflection/` are comparison material, not a claim
  that every imported utility belongs in the final storage engine.
- `rust/`: Cargo-recognized Rust scratchpad for the future storage manager.
  A learning backend now owns byte blocks and exposes a standalone C copy ABI;
  it is not the Vexspoke allocator or a production storage manager.
- Owner and tooling tests live in the independent workspace `tests/relational-engine/`.

The imported C reference retains Vexspoke's Boost Software License in
`src/LICENSE`; the repository's original MIT license remains at the root.

The intended production boundary is Rust-owned allocation/chunk lifetime with
native C processing over explicitly borrowed spans. This learning implementation
uses copy-in/copy-out instead; a zero-copy production boundary remains proposed.
See `rust/README.md` and run `python3 tests/relational-engine/rust/run.py`
from the workspace root for the owner/ABI suite.

## CLion: CMake is IDE metadata only

Open the repository root as a CMake project. `CMakeLists.txt` gives CLion C23
source targets, include paths and compiler flags for navigation, diagnostics
and inlay hints. Targets are excluded from the default build. This file is
not the release build or a new build-system dependency; no Cargo invocation,
dependency downloads, linking or application runner is wired into it.

An optional `VEXSPOKE_SOURCE_DIR` points to a local Vexspoke `src/` checkout
for remaining header references. Missing headers remain real IDE errors;
no fake declarations are generated. The imported C currently depends on
`nio/mem.h`, which is absent here and in the inspected Vexspoke checkout.
The C reference is not currently standalone-buildable.

Rust analysis requires CLion's Rust support and `rust/Cargo.toml`; CMake
does not provide Rust semantic analysis. Open/attach that Cargo package if
the IDE does not discover it automatically. IDE appearance is user-verified.

The actual build entry remains [b](https://github.com/vex-graph/b); with a
standalone b installation:

```sh
b build cargo rust
```

That builds only the Rust learning backend. It does not build the C reference
or prove the engine. No b configuration selects the CMake adapter.

## Verification and known gaps

From the workspace root, run `python3 tests/relational-engine/scaffold_test.py`
for layout, mixed-ignore behavior,
CMake metadata generation and a warnings-denied Cargo scaffold check.
These are tooling checks, not behavioral proof for the imported C classes.
The separate Rust suite exercises byte ownership, atomic byte/string publication
and an opt-in Vexspoke extern handshake through the standalone C ABI.
C dependency closure, C/Rust allocator equivalence, zero-copy borrowing,
complete concurrency/fault coverage, performance, automatic record schema
migration, live Hotcwap reload integration and Windows execution remain unproved.

Universal architecture and proof follow the linked constitution in
`CONTRIBUTING.md` and `relational-engine-preferences.md`. The engine remains an
optional backend, not a replacement for Vexspoke's current allocator. Engine
code/storage remain resident across consumer reloads; value replacement is not
automatic record schema migration.
