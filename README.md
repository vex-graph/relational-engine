# relational-engine
R2 memory/storage owner with Rust-owned spans and native C search.

**Ecosystem repository:** https://github.com/vexgraph-ecosystem/relational-engine
The local checkout is `ecosystem/repos/relational-engine`; its physical location
does not change its R2 rank. This is a partial backend, not a finished engine.

## R2 responsibility split

R2 has two cooperating repositories. Vexspoke owns CPU computation, math,
algorithms, synchronization and behavior APIs. Relational Engine owns memory
allocation/storage, stable row chunks, variable bindings and native C search
over Rust-owned spans. Production engine code includes no consumer/host headers;
imported C comparison files do not prove standalone runtime dependency closure.

Production IO/NIO ownership has migrated: the default allocator and file/cache/
log/transport implementations now live in this engine's `src/nio` and `src/io`.
Vexspoke contains no copies. Canonical includes and the default workspace build
resolve this engine-owned C ABI. Native allocation semantics and 16-byte headers
are preserved, not rewritten into Rust. Broader Rust/container migration remains
staged. R3 may borrow either R2 public contract. R1 owns
lifetimes/residency and excludes active users before destruction. GPU shaders,
dispatch, capabilities and synchronization remain Graphvex R3. No C/Rust
atomic-layout compatibility or automatic record-schema migration is assumed.

## Starting point: two interpretations

Relational-engine is an **R2 storage backend alongside Vexspoke**, not a host or
graphics driver. Rust modules separate `nio`, `io`, `primitives`, `variable`,
`struct`, `compress`, and `virtual`. Memory/string/FFI, stable chunks, append-only
collections, variable slots/registry and native C name search have implementations.
mmap belongs in `nio`; file access/writes, buffered readers/writers and FFF-style
gathering/indexing belong in `io` as planned work. Manifest-backed persistent
objects remain discussion, not implemented durable snapshots or concurrent file
writes. Graphics compute (GPU shaders/dispatch) remains Graphvex-owned.
C `src/search` owns the native search boundary; see its README and the current
owner suite for implemented operations. Rust `Chunk<T>`/`ChunkedList<T>` own
stable row allocations; `VariableSlot` is a 32-byte `repr(C)` name/value record
(`[u8; 24]` plus a borrowed value pointer), not Vexspoke StringSlot's intrusive
self-pointer layout. `VariableRegistry` is exclusively mutated and append-only;
native `re_name_search` searches initialized name bytes without reading value
pointers. Proof and limits live in the owner suite. Imported reflection/relational
files remain comparison material; migrated native IO/NIO is production code.

- `src/nio`, `src/io`: migrated production native C allocation, scratch, filesystem,
  cache, logging and bounded transport/job APIs. The macOS clipboard adapter is
  engine-owned too. Vexspoke supplies CPU-only spin/crypto/annotation/type contracts.
- `src/relational`, `src/reflection`: preserved C comparison material, never part
  of the production native target or a source of shadow consumer headers.
- `rust`: Cargo-recognized Rust scratchpad for the future storage manager.
  A learning backend now owns byte blocks and exposes a standalone C copy ABI;
  it is not the Vexspoke allocator or a production storage manager.
- Owner and tooling tests live in the independent workspace `../../../tests/relational-engine`.

The imported C reference retains Vexspoke's Boost Software License in
`src/LICENSE`; the repository's original MIT license remains at the root.

Rust owns allocation/chunk lifetime. Byte operations use copy-in/copy-out; the
native C name search now reads explicitly borrowed Rust-owned row spans. That
limited borrowing is not a complete zero-copy production engine, shared mutation,
whole-record atomicity or Vexspoke container migration.
See `rust/README.md` and run `python3 tests/relational-engine/rust/run.py`
from the workspace root for the owner/ABI suite.

## CLion: CMake is IDE metadata only

Open the repository root as a CMake project. `CMakeLists.txt` gives CLion C23
source targets, include paths and compiler flags for navigation, diagnostics
and inlay hints. Targets are excluded from the default build. This file is
not the release build or a new build-system dependency; no Cargo invocation,
dependency downloads, linking or application runner is wired into it.

An optional `VEXSPOKE_SOURCE_DIR` points to a local Vexspoke `src` checkout
for remaining header references. Missing headers remain real IDE errors;
no fake declarations are generated. `src/nio/mem.h` is now present locally.
Production IDE targets are `engine_nio`, `engine_io` (including ObjC ARC clipboard)
and `engine_search`; `reference_relational`/`reference_reflection` are reference-only.
Native IO/NIO borrows the supplied Vexspoke CPU-only include
directory. Cargo compiles native name search for Rust spans; the workspace build
compiles migrated native IO/NIO. Metadata compilation is not runtime proof.

Rust analysis requires CLion's Rust support and `rust/Cargo.toml`; CMake
does not provide Rust semantic analysis. Open/attach that Cargo package if
the IDE does not discover it automatically. IDE appearance is user-verified.

The actual build entry remains [b](https://github.com/vex-graph/b); with a
standalone b installation:

```sh
b build cargo rust
```

That builds only the Rust learning backend. The integrated production native
entry is `./tools/b build relational_engine` from the workspace root, with the
same engine dependency supplied to ordinary Vexspoke/Graphvex/Darling builds.
No b configuration selects the CMake adapter.

## Verification and known gaps

From the workspace root, run `python3 tests/relational-engine/scaffold_test.py`
for layout, mixed-ignore behavior,
CMake metadata generation and a warnings-denied Cargo scaffold check.
These are tooling checks, not behavioral proof for the imported C classes.
The separate Rust suite exercises byte ownership, atomic byte/string publication
and a separate engine extern handshake through the standalone C ABI. The stable
row slice has eleven registered Rust owner targets (sixteen test cases in each
debug/release run), real C registry/native-search clients, C-client ASan/UBSan,
exact invalid-span diagnostics and intended arity/type/borrow compile failures.
See the current checklist for executed commands and hashes, not a remembered
green. The registry C API exports `re_variables_new/drop/add/find/slot/set_pointer`
(each with the `re_variables_` prefix), with read-only borrowed slot views and
caller exclusion around rebinding/destruction. It owns labels, never their values.
C comparison dependency closure, Rust/native allocator equivalence, general zero-copy storage,
complete concurrency/fault coverage, performance, automatic record schema
migration, live Hotcwap reload integration and Windows execution remain unproved.

Universal architecture and proof follow the linked constitution in
`CONTRIBUTING.md` and `relational-engine-preferences.md`. The engine is the
production native IO/NIO owner; its Rust typed pools are a separate API. Engine
code/storage remain resident across consumer reloads; value replacement is not
automatic record schema migration.

Run `python3 tests/relational-engine/native_run.py` for strict optimized native
owners and ASan/UBSan with assertions active. Eleven owners execute in each
configuration; the clipboard mutation owner explicitly skips without permission.
HotFileSys remains a draft no-op, not an implemented watcher. Native owner proof
also covers scratch overflow rejection and child-table reap/reuse accounting.
See the checklist for exact scope; no Windows, performance or live reload claim.
