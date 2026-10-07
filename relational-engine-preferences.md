# relational-engine — Repo-Local Living Preferences

## Constitution Link

The complete [workspace constitution](https://gist.github.com/vex-graph/4132a6c45cb6d3797c3e8eff2e94035a)
governs this repository. Local workspace path: `../../preferences.md`.
Read it first, then this file, `../../tests/test-preferences.md`, current
`../../tests/test-checklist.md`, and the actual implementation.

## Law Index (Binding Matrix)

| Law Title | Scope | Enforcement |
| :--- | :--- | :--- |
| Resident Storage Boundary Law | C/Rust ownership and consumer reload | Owner/ABI/lifetime tests; no engine unload with live users |
| Cross-Language Atomic Access Law | Shared primitive values and strings | Typed atomic operations, snapshot/concurrency tests |
| One Rust Class Per File Law | Rust declarations | One struct/class or enum per implementation file, including private helpers |
| CamelCase Rust Constructor Macro Law | Rust construction spelling | CamelCase!() macros; snake_case ordinary operations |
| R2 Responsibility Layout Law | Rust storage and C search | Module/layout owner checks; explicit unimplemented scope |

### R2 Responsibility Layout Law

Relational-engine is an R2 storage backend alongside Vexspoke, not R1 and not
a GPU driver. Vexspoke may consume its opt-in C ABI; default allocation is unchanged.
Rust `nio/` owns buffers, heap/foreign storage and future file-backed mappings
(`MappedFile`/mmap); `io/` owns file access, gathering, indexing and watching.
FFF means the file-search toolkit at https://github.com/dmtrKovalenko/fff,
not a new file format; our equivalent remains future work.
Rust `primitives/` owns byte/number/string values; `variable/` owns named bindings;
`struct/` owns collections and relational records. `compress/` is explicit codec
work (ZIP/7z archives, ASTC textures), never transparent active-RAM compression.
`virtual/` is reserved for GPU-storage references and transfer contracts.
Rust paths for reserved words use `r#struct` and `r#virtual`.
Graphics compute means GPU shaders and their execution; its implementations live
in Graphvex, including dispatch, capabilities and synchronization. No engine
`compute/` implementation duplicates that owner. CPU atomic publication does not
prove GPU completion or shared-memory accessibility.

C `src/search/primitives/` is reserved for native primitive-span search over
Rust-owned bytes; imported C reference directories are not relocated or promoted
to production by this organization. These new areas have no runtime API until
implementation and registered owner proof exist. Preserve legacy `text` and root
Rust aliases while moving existing string implementation to `primitives/`.

### CamelCase Rust Constructor Macro Law

Constructor macros use CamelCase matching the object concept: `Memory!()` and
`Bytes!()`. Ordinary Rust methods and C-facing primitive operations use
snake_case, such as `get_atomic_byte`. Compatibility lowercase macros may remain
for existing clients but do not replace the canonical constructor spelling in
new examples. A byte-view constructor borrows bytes; it does not imply allocation.

### One Rust Class Per File Law

One file per class: a class is its Rust struct plus its owning `impl` methods.
The lowercase snake-case file name matches the class (Memory in `mem.rs` keeps
the established memory-module spelling). Each Rust implementation file defines
one named struct or enum at most. Keep the owning type's methods in its `impl`
in that same file; public methods still use `pub fn` so
clients can call them. Each enum, including MemoryError and private Value, has
its own file. Private helper records also get their own files. `mod.rs` and
`lib.rs` declare/re-export modules and macros, not mixed behavioral types.
Procedural FFI files may declare several operations but no owning class.

### Resident Storage Boundary Law

The engine is an optional lower-level implementation backend for Vexspoke and
database consumers, not their supervisor. Production engine code includes no
Vexspoke, Darkbase or Hotcwap headers. Imported `src/` C reference code retains
legacy Vexspoke dependencies and is not a standalone runtime implementation.

R1 keeps engine code and storage resident while consumer code reloads. Owners
belong to the host lifetime, never unloadable consumer globals. Stop admission
and finish active reads/writes before destruction. Static linking is the current
tested form; actual Hotcwap registration and live two-module reload remain gaps.

Whole-string replacement may change its byte length without invalidating prior
Rust snapshot borrows. This is value replacement, not record-schema migration.
Future schema upgrades must validate/prepare a replacement off the hot path,
publish only after successful migration and preserve the previous state on
failure; no automatic schema migration is claimed by the present implementation.

### Cross-Language Atomic Access Law

Rust owns atomic storage; C calls the canonical engine C ABI and never casts
Rust atomics to C `_Atomic` representations. Registration/release/destruction
require exclusive access. Existing atomic values permit concurrent atomic
get/set while the registry stays frozen. Non-atomic getters cannot read atomic
cells; wrong storage kinds reject with a defined error.

Atomic Bytes use release stores/acquire loads. Atomic strings publish immutable
byte snapshots with release/acquire; readers never lock. Writers use try-lock
and return busy instead of waiting. Old snapshots survive until owner destruction.
The caller-selected retention budget includes byte lengths and snapshot headers;
exhaustion is observable and leaves the current value intact. String mutation is
cold and may allocate. No lock-free writer, allocator-OOM recovery, hot-path
registry lookup, UTF-8-only storage or whole-record atomicity is claimed.

The teaching block IDs are allocation handles, not ecosystem type IDs. The
existing project-scoped type format and self-describing header are preserved
contracts for a future backend; this prototype does not implement them yet.

## Proof

Owner tests live in the independent `../../tests/relational-engine/` checkout.
The optional Vexspoke header has its own C owner in `../../tests/vexspoke/nio/`.
Use registered runners and the Timestamped Test Checklist Law; current platform,
sanitizer and migration gaps remain explicit, not production readiness.
