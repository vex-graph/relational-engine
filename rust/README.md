# Rust interpretation

## Current State

R2 storage backend with opt-in C interfaces, not a consumer rewrite or native
Memory allocator replacement. The new aligned byte-row RowPool and repaired typed
handles have macOS arm64 debug/release Rust owners and real C-client proof through
`python3 tests/relational-engine/rust/run.py`. Rust ASan currently skips for missing
compatible runtime; Windows, macOS 14 runtime compatibility, performance and
downstream R3–R5 migration remain unproved.

C includes `nio/relational_rows.h` (engine src + rust/include paths) and links
the Cargo static library. See `include/relational_engine/row_pool.h` for exact
geometry, statuses, borrow lifetime, serialization and teardown contracts. Storage
is aligned flat bytes, not Box-per-row or arbitrary Rust object construction.

This package implements a small learning backend, not a port of Vexspoke's
allocator. `src/nio/mem.rs` owns heap-backed byte blocks, `src/primitives/string.rs`
projects UTF-8 Bytes, and `src/ffi/memory.rs` exposes an opaque C owner with
copy-in/copy-out operations. Each directory's `mod.rs` declares its Rust module.
The byte-memory API exports no block pointer; the new registry API exports
read-only borrowed slot views. Single-owner calls require external exclusion.

The public module paths are `nio::mem`, `primitives::string`, and `ffi::memory`.
`text::string` and `text::atomic_string` remain compatibility re-exports.
Each named type has its own file: `nio/mem.rs` contains just `Memory` and its
methods; `nio/memory_error.rs` contains `MemoryError`. Private Block, Value,
Snapshot and History records/enums also live in individual files. `pub fn`
means a method is callable; it does not declare another class.

## Responsibility layout (R2)

Relational-engine is an R2 backend alongside Vexspoke. Existing memory/string/FFI
and stable row/binding code is implemented; other modules retain explicit planned scope:

- `src/nio`: memory and stable typed Chunk; mmap/foreign storage remain planned.
- `src/io`: file reads/writes, buffered readers/writers, gathering, FFF-style
  indexing/search and watching (planned). Manifest-backed persistent objects are
  proposed, not implemented; serialized identity must use IDs/offsets, not pointers.
- `src/primitives`: implemented byte-backed string projection and atomic snapshots.
- `src/variable`: 32-byte VariableSlot and append-only VariableRegistry.
- `src/struct`: stable ChunkedList (Rust path `r#struct`); broader collections planned.
- `src/compress`: ZIP, 7z and ASTC codecs (planned, no transparent RAM compression).
- `src/virtual`: GPU-storage/transfer contracts (planned; Rust path `r#virtual`).
- `src/ffi`: implemented opaque-owner C ABI.

Graphics-compute shaders and execution belong to Graphvex. No GPU driver is added
to this crate. C name search lives in `../src/search/primitives`; Cargo builds
only that module with strict C23 flags, using CC/AR when supplied. The upstream FFF toolkit
is https://github.com/dmtrKovalenko/fff, not an on-disk format.

## Stable rows and 32-byte bindings

```rust
use relational_engine_scratchpad::{ChunkedList, VariableRegistry};
let mut values = ChunkedList!(u64, 64).unwrap();
values.add(42).unwrap();
let gold = values.get(0).unwrap() as *const u64;
let mut names = VariableRegistry!(64).unwrap();
names.add(b"Gold", gold.cast()).unwrap();
values.add(99).unwrap(); // Existing row allocation does not move.
assert_eq!(names.find(b"gold").unwrap(), Some(0));
// names stores a borrowed pointer: keep values alive while any consumer uses it.
```

`Chunk!(T[, capacity])` owns one fixed-capacity aligned row allocation.
`ChunkedList!(T[, rows_per_chunk])` grows its directory without moving rows.
`VariableSlot!()`, `VariableSlot!(name)`, `VariableSlot!(name, pointer)` keep one
class per file. Slot layout is `[name: u8[24]][pointer: 8 bytes]`, not an intrusive
self link. Names fold lowercase, reject invalid/overlong Bytes, and preserve state
on rejection. `VariableRegistry!([rows_per_chunk])` rejects duplicate folded names
and calls real engine C search over each initialized leaf. Native lookup is cold.

`include/relational_engine/variable_registry.h` exposes new/drop/add/find/slot/
set_pointer externs. Its read-only C slot view and native Rust rows have checked
size/offset parity. Growth keeps issued slot addresses valid until owner drop;
the value address is borrowed and may be null. C callers exclude all reads before
rebinding/destruction. No stale-pointer/type validation, removal/reuse, shared
mutation, stable indirection cell, schema migration or Vexspoke container migration
is claimed. New chunk/directory allocation has deterministic OOM/retry tests;
registry owner Box and cold formatted projections still follow abort-on-OOM policy.
MSVC build and Windows execution remain unproved; native clang/GNU-style toolchains
are the current C build path. Cross builds require explicit CC and matching AR.
Root `Memory`, `Memory!()`, `mem`, `string`, and `ffi::re_memory_*` remain
available for existing clients. Shared tests mirror these module directories.
Constructor macros are CamelCase: `Memory!()` and `Bytes!()`. Lowercase
`memory!()` and `bytes!()` remain compatibility spellings, not the main examples.

```rust
use relational_engine_scratchpad::{Memory, Bytes};
let mut memory = Memory!();
let hello = memory.copy_bytes(Bytes!("hello"))?;
assert_eq!(memory.get(hello)?, b"hello");
# Ok::<(), relational_engine_scratchpad::MemoryError>(())
```

`Memory!()` is the constructor macro; `Memory::new()` remains its underlying
method. The type and macro share a name in separate Rust namespaces, so the
single import above provides both. `Bytes!` borrows bytes without allocation;
`copy_bytes` owns a copy. Release/clear invalidate IDs; IDs never repeat within
one owner. Independent owners have independent ID spaces. Rust borrows prevent
mutation during a read. C callers must obey the header's lifetime contract.

From the workspace root, run `python3 tests/relational-engine/rust/run.py`.
The independent shared suite has its own Cargo manifest; production Cargo
builds do not require the tests checkout. Running the owner suite does.
It runs the registered
Cargo tests, builds a static library and executes an actual warnings-denied C23
client, in debug and release, with a borrow-checker compile-negative case and
C-client ASan/UBSan. Rust internals are not sanitizer-instrumented by that C run.
Errors use Rust Result or C status codes, not Vexspoke THROW diagnostics.

## Reusable typed storage (first slice)

`TypedChunk!(T[, capacity])` stores actual objects at Rust's aligned `size_of::<T>()`
stride, with one occupancy bit per slot. `TypedPool!(T[, rows_per_chunk])` starts
with no backing chunks and allocates on demand. The named default is 1,024 objects
per chunk, not a total capacity limit; callers may choose smaller geometry for
large objects. Existing `Chunk`/`ChunkedList` and variable registries stay append-only.

`add(value)` returns a reusable pool-local location. `remove(index)` transfers
the value out; dropping that result destroys it. Holes are reused before new
chunks are allocated, without moving surviving objects. `release_empty_chunks()`
reclaims empty backing allocations while retaining stable directory positions.
Geometry rejects zero, zero-sized types and layout overflow. Allocation failures
drop the rejected incoming value but preserve all live values, locations and
addresses; retry is supported. Allocated-hole reuse does not allocate.

Mutation requires exclusive access. Rust references enforce this statically;
raw-pointer callers must separately exclude borrowers before removal or teardown.
These locations are NOT generation-tagged handles: a reused index can name a
different object. No stale raw-pointer rejection is claimed. This is typed Rust
storage, not yet a type registry or compatible C allocator boundary.

Separate add_handle/get_handle/get_handle_mut/remove_handle operations validate
owner-local generations: zero is invalid, exhaustion retires slots, and reclamation
retains identity metadata. The new RowPool byte API adds explicit owner identity
for C callers; it is not a compatible native Memory allocator.

The proposed 16-byte identity slots/2 MiB identity chunks,
1 MiB optional-name chunks, bulk and scratch APIs remain future work. Identity
metadata encoding is deliberately not finalized. No compaction, automatic schema
migration or concurrent allocation is introduced. Two registered typed owner
targets test bitmap edges, over-alignment, drop ownership, deterministic growth
failure/retry, reclamation, 20,000 seeded model operations and borrow rejection.
Rust sanitizer instrumentation and other-host execution remain gaps.

## Byte and atomic operations (existing API)

`get_byte(id, index)` reads ordinary stored Bytes. `get_atomic_byte(id)` reads
a registered atomic byte; `set_atomic_byte(id, value)` publishes its value.
`new_atomic_string(bytes, retention_limit)` creates a snapshot-backed byte
string. `get_atomic_string(id)` borrows one complete version and
`set_atomic_string(id, bytes)` publishes a replacement, including a new length.
Wrong storage kinds reject; an ordinary getter never reads an atomic cell.

Register cells before sharing the owner. Concurrent atomic calls may then use
the immutable registry; release/clear/drop or further registration require
exclusion. Byte loads/stores use acquire/release. A string reader takes one
acquire pointer load inside AtomicString, with no lock or allocation. Memory's
handle lookup is still a linear scan, not a proved hot-path resolver. Writers
may allocate, use try-lock and report Busy instead of waiting. Old strings are
retained until owner destruction; the explicit caller budget rejects excessive
retention without changing the current value. This is byte storage, not UTF-8
validation, record-schema migration, or atomicity across multiple fields.

The canonical C declarations are in `include/relational_engine/memory.h`;
`include/relational_memory.h` remains a compatibility include. The
engine-owned `nio/relational_memory.h` exposes these externs when supplied the
engine include path and linked to the resident engine static library. Rust/C
atomic layouts are not shared. String gets copy one complete snapshot into a
caller buffer, with an explicit truncation flag on insufficient capacity.

Hotcwap must keep the engine and owner state resident across consumer reloads,
and quiesce calls before destruction. No Hot loader code has been changed or
live-reload integration proved. String length replacement works; automatic
record layout upgrades still require a staged migration/rollback design.

Known gaps in this Rust byte API: no slabs, BitPool, typed record macros, 16-byte native headers,
pointer compatibility, exhaustive legacy Memory OOM injection, shared registry
mutation, performance or Windows
proof, deterministic writer-busy fault injection or full Rust sanitizer proof.
The constructor follows Rust's abort-on-OOM Box policy. Vec-to-box
conversion may allocate. These cold operations are not real-time safe. Vexspoke
CPU consumers now use engine-owned native IO/NIO; this separate Rust byte API
is not an automatic replacement of their allocator or a completed R5 migration.

## Scope and Limitations

Rust owns selected storage and exposes C operations without converting consumers
to Rust. This slice does not provide database transactions, disk durability,
ecosystem type/header parity, hot allocation guarantees, internal shared mutation
or automatic schema migration. Caller serialization and resident-engine lifetime
are mandatory; arbitrary/stale pointers are outside the C ABI contract. Current
toolchain sanitizer and macOS floor gaps are stated in Current State.
