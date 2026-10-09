# relational-engine — Repo-Local Living Preferences

## Constitution Link

The complete [workspace constitution](https://gist.github.com/vex-graph/4132a6c45cb6d3797c3e8eff2e94035a)
governs this repository. Local workspace path: `../../../preferences.md`.
Read it first, then this file, `../../../tests/test-preferences.md`, current
`../../../tests/test-checklist.md`, and the actual implementation.

## Law Index (Binding Matrix)

| Law Title | Scope | Enforcement |
| :--- | :--- | :--- |
| Resident Storage Boundary Law | C/Rust ownership and consumer reload | Owner/ABI/lifetime tests; no engine unload with live users |
| Cross-Language Atomic Access Law | Shared primitive values and strings | Typed atomic operations, snapshot/concurrency tests |
| One Rust Class Per File Law | Rust declarations | One struct/class or enum per implementation file, including private helpers |
| CamelCase Rust Constructor Macro Law | Rust construction spelling | CamelCase!() macros; snake_case ordinary operations |
| R2 Responsibility Layout Law | Rust storage and C search | Module/layout owner checks; explicit unimplemented scope |
| Stable Row and Variable Binding Law | Chunk/registry lifetimes and 32-byte ABI | Growth, layout, failure and C-client owner tests |

### Stable Row and Variable Binding Law

`nio/chunk.rs` owns a fixed-capacity typed row allocation; `struct/chunked_list.rs`
grows its directory without moving initialized rows. Geometry is chosen at
construction (named default, caller override), validated for size/alignment and
immutable thereafter. Rows are append-only, never deleted/reused in this version.
Growth and mutation require exclusive access; this is not a concurrent radix
publication implementation. Raw row addresses survive growth but not destruction.
Failed admission preserves initialized rows/length; rejected owned input is dropped.

`variable/variable_slot.rs` is a C-layout 32-byte binding: `[name: u8[24]][value
pointer: 8 bytes]`, with offsets 0/24 checked. Names are 1..23 ASCII bytes plus
NUL and zero padding, folded lowercase under the existing segmented-name grammar.
Invalid input rejects without silent truncation or mutating prior state.
This pointer is borrowed VALUE storage, not the legacy `StringSlot.self` link;
the two records must never be conflated. No dereference/ownership/type validation
is implied by storing a pointer. Zero slots are unnamed/unbound; registry admission
requires a valid name. Plain binding replacement is exclusive, not atomic.

`VariableRegistry` owns stable slot rows and rejects folded duplicate names.
Engine-owned C `search/primitives/name_search` scans fixed 24-byte keys in borrowed
leaves. It includes no Vexspoke/host headers, owns no allocation, reports invalid
span admission through engine `exception/throw.h`, and preserves output on failure
or absence. Rust/FFI use explicit Result/status codes for admission; they do not
depend on Vexspoke THROW. The C ABI exposes an opaque registry and read-only borrowed
slot views; caller excludes reads before rebind/drop and retains pointed values.
Pointers are live-process addresses, not persistent/SSD/GPU identifiers or portable
serialization. Schema migration and a stable indirection cell remain future work.
No public free/reuse/generation check exists; using stale/non-live pointers is
outside the ABI contract, not a claimed safe rejection. The first C adapter can
be used independently; migration of any existing Vexspoke collection is not implied.

### R2 Responsibility Layout Law

The reusable typed-storage slice is separate from the append-only classes:
`nio/typed_chunk.rs` owns aligned object rows and a packed occupancy bitmap;
`struct/typed_pool.rs` owns a growable directory with lazy backing chunks.
The caller-selected default is 1,024 rows per chunk, never a total ceiling.
Exclusive removal transfers ownership out and reuses holes without moving live
objects. Explicit empty-chunk release retains directory positions and requires
excluding raw-pointer borrowers. Failed fallible growth preserves all live state
and drops the rejected incoming value. Raw pool-local indices are reusable
locations; the generation-tagged `Handle` surface (`add_handle` / `get_handle` /
`get_handle_mut` / `remove_handle`) is the identity API and rejects a reused slot
as stale. No ecosystem type registry, C allocator parity, compaction or concurrent
allocation is claimed. Optional-name integration, bulk and scratch remain future work.

Relational-engine is an R2 storage backend alongside Vexspoke, not R1 and not
a GPU driver. Engine `src/io` and `src/nio` now own the production native C
Memory/MemoryArena/Transient and file/cache/log/transport ABI. Vexspoke has no
IO/NIO implementation copies; the production build consumes the engine by
default. C allocation semantics, 16-byte headers and legacy project type IDs
are preserved, not rewritten into Rust. Existing Rust typed pools remain a
distinct API, not a silent allocator substitution. Native code may borrow
Vexspoke CPU-only spin/crypto/type/annotation contracts; never downstream or
host headers, duplicate memory implementations or a recursive target graph.
Rust `nio/` owns buffers, heap/foreign storage and future file-backed mappings
(`MappedFile`/mmap); `io/` owns file reads/writes, buffered readers/writers,
gathering, indexing and watching. Manifest-backed persistence remains proposed;
no concurrent file-commit/durability contract is implemented or implied.
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

C `src/search/primitives` contains native primitive-span name search over
Rust-owned bytes. C `src/io` and `src/nio` are promoted migrated production
implementations; `src/reflection` and `src/relational` remain comparison code.
Other planned Rust areas have no runtime API until
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

The opt-in RowPool C API owns runtime-selected aligned byte rows, not native
Memory blocks or arbitrary Rust types. C sees an opaque owner and a RowHandle
with process-local owner/index/generation/reserved fields. Zero is invalid;
generation exhaustion retires slots, and empty backing release retains identity
history. Rust TypedChunk/TypedPool handles remain owner-local. Counts, backing,
geometry and owner identity change through construction/add/remove/release only,
not raw setters that could forge lifetime or identity. This scoped managed
exception under the Conflict Triage Law and Single Class Per File Law preserves
ownership truth. Row FFI uses explicit status codes as its observable failure
channel, matching the engine's existing Rust/FFI policy, not synchronous logging.
All C calls require live disjoint spans and external serialization; borrowed row
bytes are read-only and consumers detach before removal/destruction. Ordinary
row allocation is fallible; cold projections retain Rust's abort-on-OOM behavior.
Racing owner-ID construction may reject for external retry, never loops. The
native allocator remains unchanged and default builds do not silently opt in.

The engine is the production IO/NIO implementation owner for Vexspoke and
database consumers, not their supervisor. Its migrated native IO/NIO borrows
Vexspoke CPU-only contracts under the constitution's explicit R2 seam; it never
includes Darkbase/Hotcwap or other downstream headers. Native migrated code
is compiled and linked by the default build. Other imported comparison code
does not confer standalone runtime dependency closure.

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

Owner tests live in the independent `../../../tests/relational-engine` checkout.
Migrated native IO/NIO owners live in `../../../tests/relational-engine/io` and
`../../../tests/relational-engine/nio`, including the Rust byte/string handshake.
Use registered runners and the Timestamped Test Checklist Law; current platform,
sanitizer and migration gaps remain explicit, not production readiness.

## Readiness Cross-Reference (Living Documentation Law)

- Feature readiness matrix: [relational-engine](https://gist.github.com/vex-graph/6943f92acb931b25dad1073c46da6ce7#file-relational-engine-md).
- Open blockers and deferred decisions: [ecosystem blockers Gist](https://gist.github.com/vex-graph/e921fa188eebbd0c68c4e59646109887).
