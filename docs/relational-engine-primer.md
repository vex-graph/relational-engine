# Relational Engine — a plain-English primer

This is a reading document, not a spec. It explains what the relational engine is,
why it exists, how the C and Rust halves talk to each other, and why Rust is doing
the storage work. It is written so you can follow the whole system without reading
every file, and so you can disagree with the architecture in an informed way.

If something here contradicts the source, the source wins — this document is a
map, not the territory. Source locations and the review boundary are listed below.

## Start here: is it ready to ship?

**There is no whole-engine production-readiness claim.** This repository contains
native C infrastructure used by the ecosystem and a separate, developing Rust
storage API. A working primitive, a passing owner test, and a finished storage
product are three different milestones.

The earlier description of the new handles as fully "generation-safe" was too
strong. The current implementation has specific identity defects described below.
A past green suite did not exercise those cases. This document update records a
source review, not a fresh runtime certification.

Also, RE is R2 infrastructure, not the R3 database. It need not become a database
or acquire a WAL to be useful. Database transactions and recovery policy belong to
Darkbase; RE must deliver the allocation and file primitives those policies need.

---

## 1. The one-sentence version

The relational engine is the ecosystem's **memory and storage owner**: it holds
values and offers stable-row storage plus optional **name-to-pointer bindings**.
Not every allocation is named, and address stability has an owner lifetime.

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
They cooperate through public contracts: Vexspoke consumes engine IO/NIO and type
algebra; the migrated native engine layer can borrow Vexspoke CPU-only helpers.
The split is not a claim of a completely one-directional source dependency.

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
   alignment for matching fields on the same target ABI. This does not recursively
   make arbitrary field types C-compatible. Scalars and opaque pointers also cross;
   Rust `String`, `Vec`, and data-carrying enums are not passed by value here.

3. **Opaque owners.** The Rust side owns its data. C holds a `*mut Owner` it never
   looks inside, plus functions to create, use, and drop it. This is the
   "everything is a pointer" idea at the language boundary.

4. **Status codes, not exceptions.** Rust panics must **never** unwind into C. Every
   fallible read/write operation returns a small integer status (`0 = success`, `1 = invalid`,
   `2 = allocation`, `3 = unknown id`, `4 = capacity`, …) and writes results into
   caller-provided outputs. Creation returns a pointer and destruction returns
   nothing. The current bridge has no general panic-to-status catcher; allocation
   of the owner can abort on OOM. `extern "C"` is not exception recovery.

5. **Caller-owned output, dest-last.** Data flows *out* through pointers the caller
   supplied. A string read takes `(owner, id, dest, capacity, out_length,
   out_truncated)` — the destination and flags come last.

6. **Explicit ownership and aliasing rules.** Follow each function's pointer and
   exclusion contract. Creation, copying and string replacement can allocate.
   C returns Rust-owned owners to their Rust destructor, never to C `free`.

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
| fixed-width ints (`u32`, `u64`), target-sized `usize` / C `size_t` | trait objects, closures |
| function pointers with C signatures | Rust enums with data |
| explicitly agreed records such as `VariableSlot` | Rust atomics reinterpreted as C `_Atomic` |

---

## 6. Why Rust is here (it does not have to be)

The same design could be implemented in C. Rust is a tradeoff: more compiler help
inside the storage owner, in exchange for a second language and a maintained ABI.

- **No mandatory garbage collector.** Rust supports explicit allocation and
  destruction. `Vec`, `Box` and `format!` still allocate; Rust does not automatically
  make a path allocation-free or realtime-safe.
- **Ownership and borrowing** catch use-after-free, double-free, and aliasing bugs
  in safe Rust before the program runs, provided unsafe implementations uphold
  their contracts. The compiler cannot validate arbitrary pointers supplied by C.
- **`MaybeUninit`, explicit `unsafe`, and `Drop`** let the engine build manual,
  bit-level storage while keeping the unsafe surface small and reviewable.
- **Atomics are first-class**, as they also are in C. Correct publication and
  lifetime protocols still require design and concurrency tests.
- **Type safety at the boundary**: the compiler proves the Rust side's invariants,
  but the unsafe bridge remains manually reviewed and tested.

Costs include compiler/toolchain integration, explicit unsafe code, header drift,
and restrictions on aliasing. C would simplify language integration but shift more
lifetime enforcement into conventions, review and tests. Neither language proves
identity correctness, crash durability or performance. The handle defects below
are examples of logic errors that Rust can compile without complaint.

---

## 7. The data model (the pieces)

### 7.1 The 16-byte block header

The native C `Memory_*` allocation contract uses a 16-byte header:

```
[ typeId: u64 ][ length: u32 ][ sugar: u32 ]   then the payload bytes
```

The payload pointer sits 16 bytes after the header, so the header is found by one
subtraction. `typeId` says what it is; `length` says how big; `sugar` is a
recognizable marker. This is allocation metadata, not permission to inspect an
arbitrary address. The separate Rust `Memory` and typed pools do **not** implement
this header ABI; moving native C files into RE did not rewrite them in Rust.

### 7.2 Memory (`Memory`) — the byte store

A simple owner of byte blocks. You copy bytes in, get an id back, read them out.
Strings can be atomic (whole-value snapshots with release/acquire publication) so
readers never lock.

### 7.3 Chunks and pools — stable rows

- **`Chunk<T>`** — a fixed-capacity block of typed rows that never moves.
- **`ChunkedList<T>`** — a growable directory of chunks; initialized rows retain
  their addresses across growth, until owner destruction.
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

Every slot carries a generation bumped on removal. Access compares the supplied
generation to the slot's current generation and checks that the slot is occupied.
This rejects ordinary remove/reuse cases, but **permanent stale rejection is not
currently delivered**:

- `Handle::zero()` is `(0, 0)`, and the first insertion into a fresh chunk issues
  that same pair. The documented invalid sentinel can name a live value.
- `TypedChunk::bump_generation` wraps back to one. Eventually a previously issued
  generation can repeat; skipping zero does not prevent resurrection.
- `TypedPool::release_empty_chunks` discards generation metadata. Recreating the
  chunk starts at zero again, so an old handle can resolve to a new value.
- Handles contain no owner identity. A handle from another pool can match; callers
  must keep handles associated with their original owner.

These findings come from `rust/src/nio/{handle,typed_chunk}.rs` and
`rust/src/struct/typed_pool.rs`. The existing owner tests exercise ordinary reuse,
but not sentinel rejection, generation exhaustion or stale handles after chunk
recreation. Raw pointers are a separate obligation: handle checks do not make a
previously escaped pointer safe after removal.

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
R2 relational engine -- separate contracts

  Native C Memory_*: existing allocator + 16-byte header
  Rust Memory: byte blocks + atomic values, separate opaque C ABI
  Rust TypedPool: reusable rows + draft generation handles (Rust API)
  Rust VariableRegistry: stable labels + borrowed value pointers
       |
       +-- calls native C name search over its slot rows

A registry pointer may refer to caller-owned storage; it is not automatically
connected to a Rust Memory block or TypedPool handle.
```

---

## 9. What is real vs planned (honest)

**Separate surfaces:** native C `Memory_*`, Rust `Memory`, reusable typed pools,
and named bindings are not one interchangeable allocator. This review read the
Rust memory bridge, registry and typed-handle implementations. Other native IO
owners need their own current proof; this document does not certify the whole
family or clipboard behavior.

**Mapping status at this review:** no file matching `*mapped*` was found in the
engine checkout. That inventory check is not a claim about another agent's work
or every possible implementation name. MappedFile remains a dependency to verify
before planning a consumer around it. Broader planned areas are described in the
repo preferences; their implementation is not established by this document.

**Evidence boundary:** the checklist records earlier macOS owner runs, not a new
run for this documentation update. C-client ASan/UBSan does not instrument Rust.
Windows execution, Rust sanitizer instrumentation, full fault/concurrency coverage
and measured performance are not established here. Neither a crate's name nor a
green summary is a readiness decision.

---

## 10. Where you might want to change the architecture

Honest pressure points, so you can push on them:

1. **The ABI is manual.** Every crossing function restates its contract in a comment.
   A generated header could reduce declaration drift; ownership and lifetime
   contracts would still require review and C-client tests.
2. **Handles are not yet the C ABI.** Identity currently lives in the Rust API; C
   callers use the separate memory/registry contracts. Repairing handle identity
   comes before exposing that typed API to C.
3. **Indices vs handles.** Keeping both raw indices and handles is pragmatic but
   leaves a footgun. If you want identity everywhere, the raw index API could be
   demoted to `pub(crate)`.
4. **One header, one truth.** The 16-byte block header lives with the allocator; the
   type algebra lives in `src/type`. They are related but separate — worth deciding
   whether the header should carry more self-description.
5. **Mapping is a choice, not a readiness badge.** mmap exposes file pages through
   addresses; it does not supply transactions or crash recovery. Buffered IO may
   be sufficient for a chosen workload. Page faults can block, so mapped storage
   must not be assumed to satisfy a hot-path latency bound.

### A practical roadmap, in dependency order

1. **Repair typed identity.** Reserve a truly invalid sentinel, retain identity
   history across backing release, and define non-resurrection on exhaustion.
   Decide whether wrong-owner rejection is offered or caller responsibility.
   Prove these cases alongside allocation failure and preserved live state.
2. **Choose the first consumer and its contract.** State what it stores, how long
   pointers live, who may mutate it, and what happens when capacity or allocation
   fails. In-memory storage can be useful without any disk persistence.
3. **Expose only the required C surface.** Define ownership, layout, status codes,
   panic/OOM behavior and teardown, then exercise actual C callers. Generic Rust
   pools do not automatically become a generic C allocator.
4. **Add file-backed storage if that consumer needs it.** Prove file lifetime,
   mapping or IO failures, bounds, flush behavior and teardown. Darkbase then owns
   its database publication/recovery policy. A WAL is one possible strategy;
   copy-on-write or atomic snapshot publication may suit a different contract.
5. **Prove the intended deployment.** Run owner and integration tests, applicable
   sanitizers, fault injection and representative benchmarks on supported targets.
   macOS and Windows proof are separate; Linux is not an automatic release gate
   imposed by this tutorial. Prove host residency/reload if the consumer uses it.

Each step closes a specific promise. None authorizes a blanket "production-ready"
label for unrelated parts of the repository.

### The distinctions to keep in your head

- **Address:** where bytes are now. Stable across growth does not mean immortal.
- **Identity:** which object you mean, even when storage locations get reused.
- **Name:** a lookup label. `VariableRegistry` stores a borrowed pointer; it does
  not own, type-check or keep the pointed-to value alive.
- **Persistence:** bytes survive process exit. A pointer's numeric address does
  not become meaningful in the next process by being written to disk.
- **Durability:** acknowledged changes survive the specified crash/power-loss
  model. Successful writing or mapping alone does not establish this.
- **Bytes versus interpretation:** byte storage avoids mandatory text conversion.
  Integers still have byte order; records can contain padding and pointers.
  A durable format must define what is stored and how references are rebuilt.

For example, registering `health` against caller-owned bytes only makes those bytes
findable. The caller must retain them and coordinate access. Saving that registry
record verbatim would save a process address, not a portable health value.

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

*Review boundary (2026-10-09): read the constitution and repo/test preferences,
affected checklist rows, this primer, Rust `nio/{handle,typed_chunk,mem}.rs`,
`struct/typed_pool.rs`, `variable/variable_registry.rs`, `ffi/memory.rs`, its C
header, and the typed-chunk/pool owner tests. Inspected native `MemoryHeader`
declarations and mapped-file filename inventory. This was not a complete native
IO, atomic-string implementation, Darkbase or cross-platform audit. No runtime
suite was rerun for this prose-only change.*
