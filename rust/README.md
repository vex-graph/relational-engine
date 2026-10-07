# Rust interpretation

This package implements a small learning backend, not a port of Vexspoke's
allocator. `mem.rs` owns heap-backed byte blocks, `string.rs` projects UTF-8
bytes, and `ffi.rs` exposes an opaque C owner with copy-in/copy-out operations.
No block pointer escapes to C. Single-owner calls require external exclusion.

```rust
use relational_engine_scratchpad::{Memory, bytes};
let mut memory = Memory::new();
let hello = memory.copy_bytes(bytes!("hello"))?;
assert_eq!(memory.get(hello)?, b"hello");
# Ok::<(), relational_engine_scratchpad::MemoryError>(())
```

`memory_new!()` is optional constructor sugar; `Memory::new!()` is not legal
associated-macro syntax in Rust. `bytes!` borrows bytes without allocation;
`copy_bytes` owns a copy. Release/clear invalidate IDs; IDs never repeat within
one owner. Independent owners have independent ID spaces. Rust borrows prevent
mutation during a read. C callers must obey the header's lifetime contract.

Run `python3 tests/rust/run.py` from the repository root. It runs the registered
Cargo tests, builds a static library and executes an actual warnings-denied C23
client, in debug and release, with a borrow-checker compile-negative case and
C-client ASan/UBSan. Rust internals are not sanitizer-instrumented by that C run.
Errors use Rust Result or C status codes, not Vexspoke THROW diagnostics.

Known gaps: no slabs, BitPool, typed record macros, 16-byte Vexspoke headers,
pointer compatibility, fault-injected OOM, concurrency, performance or Windows
proof. The constructor follows Rust's abort-on-OOM Box policy. Vec-to-box
conversion may allocate. These cold operations are not real-time safe. Vexspoke
remains unchanged; this backend is not wired to R5.
