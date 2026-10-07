# Rust interpretation

This package implements a small learning backend, not a port of Vexspoke's
allocator. `src/nio/mem.rs` owns heap-backed byte blocks, `src/text/string.rs`
projects UTF-8 Bytes, and `src/ffi/memory.rs` exposes an opaque C owner with
copy-in/copy-out operations. Each directory's `mod.rs` declares its Rust module.
No block pointer escapes to C. Single-owner calls require external exclusion.

The public module paths are `nio::mem`, `text::string`, and `ffi::memory`.
Each named type has its own file: `nio/mem.rs` contains just `Memory` and its
methods; `nio/memory_error.rs` contains `MemoryError`. Private Block, Value,
Snapshot and History records/enums also live in individual files. `pub fn`
means a method is callable; it does not declare another class.
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

## Byte and atomic operations

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
`include/relational_memory.h` remains a compatibility include. Vexspoke's
optional `nio/relational_memory.h` exposes these externs when supplied the
engine include path and linked to the resident engine static library. Rust/C
atomic layouts are not shared. String gets copy one complete snapshot into a
caller buffer, with an explicit truncation flag on insufficient capacity.

Hotcwap must keep the engine and owner state resident across consumer reloads,
and quiesce calls before destruction. No Hot loader code has been changed or
live-reload integration proved. String length replacement works; automatic
record layout upgrades still require a staged migration/rollback design.

Known gaps: no slabs, BitPool, typed record macros, 16-byte Vexspoke headers,
pointer compatibility, fault-injected OOM, concurrency, performance or Windows
proof, deterministic writer-busy fault injection or full Rust sanitizer proof.
The constructor follows Rust's abort-on-OOM Box policy. Vec-to-box
conversion may allocate. These cold operations are not real-time safe. Vexspoke
remains unchanged; this backend is not wired to R5.
