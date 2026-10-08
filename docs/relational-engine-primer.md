# Relational Engine — a plain-English primer

This is a reading document, not a spec. It explains what the relational engine is,
why it exists, how the C and Rust halves talk to each other, and why Rust is doing
the storage work. It is written so you can follow the whole system without reading
every file, and so you can disagree with the architecture in an informed way.

If something here contradicts the source, the source wins — this document is a
map, not the territory. Every claim below is grounded in files you can open.

---

## 1. The one-sentence version

The relational engine is the ecosystem's **memory and storage owner**: it holds
values as raw bytes, gives every value a **name** and a **stable address**, and
lets any part of the system ask "where is the thing called X right now?".

It sits at **R2** (the second tier of the R1–R5 stack). It does not own windows,
GPUs, or UI. It owns: allocation, files, stable rows, named bindings, the shared
type-id algebra, and native search over rows.

---

## 2. Why it exists (the thesis)

Most systems answer two questions badly:

- **"Process everything fast"** — data-oriented arrays. Great, but nothing is
  *findable* by name.
- **"Give me all entities with components A, B, C"** — ECS. Great for schedulers,
  but "the thing called `character.position.x`" needs a bolted-on name table.

The relational engine answers a third question: **"find the thing called X, right
now, from anywhere."** The primitive is simple:

```
name  ->  value
```

One primitive underlies many features: a debugger inspecting live state, a script
binding a variable, a save/load walk, a hot-swap rebinding a function, telemetry
reading a metric. Instead of N bespoke lookup systems, there is one.

Everything is a pointer. A value is bytes; a name points at those bytes; the
engine's job is to keep the pointers honest and the addresses stable.

---

## 3. The R2 split (two repositories, one tier)

R2 is deliberately two cooperating owners:

- **Vexspoke** — CPU *computation and behavior*: math, algorithms, collections,
  synchronization, reflection, reactive values.
- **Relational engine** (this repo) — *memory and storage*: the allocator, file
  IO, stable row chunks, named bindings, native search, the type algebra.

The rule of thumb: if it computes, it's Vexspoke; if it stores, it's the engine.
They never include each other's private headers in a cycle — the engine is the
floor, Vexspoke stands on it.

---

## 4. Two languages, one seam

The engine is **C and Rust together**, on purpose:

- **C** owns the ABI and the leaf drivers: allocation, files, caches, sockets,
  process spawn, the macOS clipboard, the shared type algebra.
- **Rust** owns the new storage core: typed chunks, pools, atomic strings, the
  variable registry, the byte/string primitives.

Why both? C is the ecosystem's lingua franca — every tier speaks it, and it links
to the OS with zero ceremony. Rust is where the *storage* logic is safest: it
catches aliasing and lifetime bugs at compile time. The trick is a clean seam
between them.

---

## 5. How C talks to Rust (the ABI, plainly)

Rust can look scary from C, but the boundary rules are simple and strict. Think of
it as: **C sees an opaque handle and a handful of `extern "C"` functions that take
plain integers and pointers — nothing else.**

The rules:

1. **`extern "C"` + `#[unsafe(no_mangle)]`.** A Rust function marked this way has a
   fixed C name and C calling convention. C calls it like any other function.

2. **`#[repr(C)]` for shared structs.** A Rust struct with `#[repr(C)]` has the
   exact same byte layout C would produce — same field order, same offsets, same
   alignment. Only `#[repr(C)]` types may cross the seam. Never `String`, `Vec`,
   trait objects, or Rust enums.

3. **Opaque owners.** The Rust side owns its data. C holds a `*mut Owner` it never
   looks inside, plus functions to create, use, and drop it. This is the
   "everything is a pointer" idea at the language boundary.

4. **Status codes, not exceptions.** Rust panics must **never** unwind into C. Every
   boundary function returns a small integer status (`0 = success`, `1 = invalid`,
   `2 = allocation`, `3 = unknown id`, `4 = capacity`, …) and writes results into
   caller-provided outputs.

5. **Caller-owned output, dest-last.** Data flows *out* through pointers the caller
   supplied. A string read takes `(owner, id, dest, capacity, out_length,
   out_truncated)` — the destination and flags come last.

6. **No aliasing, no hidden allocation.** Inputs and outputs must not overlap. The
   ABI says who allocates and who frees; C never frees Rust memory.

7. **Arbitrary pointers are not validated.** A non-null pointer must be live and
   aligned. "Everything is a pointer" means the engine trusts the pointer you hand
   it, exactly as C does — it cannot prove an address is mapped.

A concrete example, from `rust/include/relational_engine/memory.h` and
`rust/src/ffi/memory.rs`:

```c
/* C sees this. The owner is opaque; the bytes are copied in/out. */
typedef struct ReMemory ReMemory;
ReMemory *re_memory_new(void);
uint32_t  re_memory_copy(ReMemory *owner, const uint8_t *source, size_t length, uint64_t *output);
uint32_t  re_memory_read(const ReMemory *owner, uint64_t id, uint8_t *dest, size_t cap, size_t *out_len);
```

```rust
/* Rust implements it. repr(C) types, C names, integer status. */
#[unsafe(no_mangle)]
pub extern "C" fn re_memory_new() -> *mut Memory { Box::into_raw(Box::new(Memory::new())) }
```

That is the whole idea: a C header, a matching Rust `extern "C"` implementation,
opaque handles, and status codes.

### What crosses, and what does not

| Crosses the seam | Never crosses |
| :--- | :--- |
| `#[repr(C)]` structs | `String`, `Vec`, `Box` (by value) |
| raw pointers (`*const T`, `*mut T`) | Rust references (`&T`, `&mut T`) |
| fixed-width ints (`u32`, `u64`, `usize`) | trait objects, closures |
| function pointers with C signatures | Rust enums with data |
| the 16-byte block header | Rust `_Atomic` assumed as C `_Atomic` |

---

## 6. Why Rust is here (and why it "has to be")

Rust is not here for fashion; it is here because the storage core has exactly the
failure modes Rust eliminates at compile time:

- **No garbage collector, no hidden allocation.** Rust gives C-like control with
  no runtime. That matches the ecosystem's "zero steady-state allocation" doctrine.
- **Ownership and borrowing** catch use-after-free, double-free, and aliasing bugs
  before the program runs — the bugs that turn a database into corruption.
- **`MaybeUninit`, explicit `unsafe`, and `Drop`** let the engine build manual,
  bit-level storage while keeping the unsafe surface small and reviewable.
- **Atomics are first-class**, so the lock-free pieces (atomic strings, snapshots)
  are expressed directly rather than through a C macro layer.
- **Type safety at the boundary**: the compiler proves the Rust side's invariants,
  so C only has to honour the documented ABI contract.

The cost is a real seam to maintain (the ABI rules above). The benefit is a storage
core that is hard to corrupt.

---

## 7. The data model (the pieces)

### 7.1 The 16-byte block header

Every allocation the engine hands out is prefixed by a 16-byte header:

```
[ typeId: u64 ][ length: u32 ][ sugar: u32 ]   then the payload bytes
```

The payload pointer sits 16 bytes after the header, so the header is found by one
subtraction. `typeId` says what it is; `length` says how big; `sugar` is a
recognizable marker that fails closed if the header is wrong. This is how a bare
`void*` becomes self-describing — the runtime twin of the `;;WHAT("T")` annotation.

### 7.2 Memory (`Memory`) — the byte store

A simple owner of byte blocks. You copy bytes in, get an id back, read them out.
Strings can be atomic (whole-value snapshots with release/acquire publication) so
readers never lock.

### 7.3 Chunks and pools — stable rows

- **`Chunk<T>`** — a fixed-capacity block of typed rows that never moves.
- **`ChunkedList<T>`** — a growable directory of chunks; rows keep their address
  forever, so a pointer handed out once stays valid.
- **`TypedChunk<T>`** — chunk + a packed occupancy bitmap, so freed slots are reused
  without moving survivors.
- **`TypedPool<T>`** — a growable directory of `TypedChunk`s.

The point of all four: **stable addresses**. Rows do not shuffle when the
collection grows.

### 7.4 Handles — identity, not just location

A raw index into a pool is a *location*. Free the slot, reuse it, and an old index
silently points at a different object. That is the opposite of identity, so the
engine now has a generation-tagged **`Handle`**:

```
Handle { index: usize, generation: u32 }
```

Every slot carries a generation that is **bumped when the slot is freed**. A handle
captures the generation at issue, and `get_handle` / `remove_handle` reject a handle
whose generation no longer matches — a *stale* handle. Reusing a slot gives the new
value a new generation, so the old handle stays dead. That is what makes a location
into an identity. (Raw indices remain available for low-level use; the handle
surface is the identity API.)

### 7.5 Variables — name → value

A **`VariableSlot`** is a 32-byte record:

```
{ uint8_t name[24], pointer-to-value }
```

The name is a 24-byte byte span (folded, validated); the pointer addresses the
value's bytes. The **`VariableRegistry`** holds these slots in a `ChunkedList` and
searches them with native C (`re_name_search`). This is the "name → value" thesis in
concrete form.

### 7.6 The type algebra

A 64-bit id encodes *whose* class it is (project byte), its *shape* (form), and its
*class number* (per project). The algebra (`Type_make`, `Type_class`, `Type_arch`,
the parent-chain resolver) is shared and project-agnostic; each repository keeps its
own class registry. This lets one allocator serve every type across the whole stack.

---

## 8. How it fits together (picture)

```
        name  ->  value
          |         |
   VariableSlot   bytes (Memory block)
          |         |
   VariableRegistry  Chunk / TypedChunk / TypedPool
          |         |
     native C search  Handle { index, generation }  ->  identity
          \        /
        the relational engine (R2 storage)
                 |
     C ABI (repr(C) + extern "C" + status codes)
                 |
   C callers (Vexspoke CPU, R3 drivers, hosts)  <->  Rust storage core
```

---

## 9. What is real vs planned (honest)

**Real and proven (macOS arm64):** the migrated native C allocator and IO
(`Memory`, `File`, `Cache`, `Log`, `VexHome`, `ProcessSpawn`, `WsClient`,
clipboard); Rust byte/string + atomic string snapshots; `Chunk`/`ChunkedList`,
`TypedChunk`/`TypedPool`; generation-tagged `Handle`; `VariableSlot` +
`VariableRegistry` + native `re_name_search`; the shared `src/type` algebra.

**Planned, not implemented:** mmap / `MappedFile` (the engine owns it; contract
agreed, no code yet); buffered/async file IO and directory traversal; manifest-backed
persistence; codecs (ZIP/7z, ASTC); GPU storage-transfer (`virtual/`); live
Hotcwap reload integration.

**Unproven:** Windows execution; Rust sanitizer instrumentation; performance
numbers; full concurrency/fault matrix. A pass on one platform is not proof on
another.

---

## 10. Where you might want to change the architecture

Honest pressure points, so you can push on them:

1. **The ABI is manual.** Every crossing function restates its contract in a comment.
   A generated header (from the Rust `extern "C"` surface) would remove drift risk.
2. **Handles are not yet the C ABI.** Identity currently lives in the Rust API; C
   callers still see raw ids. Exposing handles over the C boundary is the next step.
3. **Indices vs handles.** Keeping both raw indices and handles is pragmatic but
   leaves a footgun. If you want identity everywhere, the raw index API could be
   demoted to `pub(crate)`.
4. **One header, one truth.** The 16-byte block header lives with the allocator; the
   type algebra lives in `src/type`. They are related but separate — worth deciding
   whether the header should carry more self-description.
5. **No mmap yet.** The store is streamed through `File`. mmap is the big storage
   upgrade (see `MappedFile`), and it is where the cold-vs-hot line matters most:
   page faults are unbounded waits, so mapping is a *storage* tier, never a hot path.

---

## 11. Glossary

- **R1–R5** — the five supervision tiers (host → storage → drivers → UI → apps).
- **ABI** — the binary contract two languages share: layout, calling convention,
  names.
- **`repr(C)`** — "lay this out exactly like C would".
- **opaque handle** — a pointer C holds but never dereferences itself.
- **block header** — the 16 bytes before every payload (`typeId`/`length`/`sugar`).
- **stable row** — a stored element whose address never changes while it lives.
- **generation** — a counter bumped when a slot is freed, so old handles go stale.
- **Handle** — `{ index, generation }`: a location plus its identity.
- **VariableSlot** — `{ name[24], pointer }`: a name bound to value bytes.
- **type algebra** — the shared 64-bit id encoding and its readers.
- **seam** — the C/Rust boundary and its rules.

---

*Read boundary: this primer is grounded in the engine's README, preferences,
`src/nio`, `src/io`, `src/type`, and the Rust `nio`/`struct`/`variable`/`ffi`
modules as read this cycle. It is a map; when it and the source disagree, the
source is right.*
