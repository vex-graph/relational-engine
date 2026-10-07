//! Learning storage layer, not a replacement for Vexspoke's slab allocator.
//!
//! ```
//! use relational_engine_scratchpad::{Memory, bytes};
//! let mut memory = Memory::new();
//! let hello = memory.copy_bytes(bytes!("hello"))?;
//! assert_eq!(memory.get(hello)?, b"hello");
//! # Ok::<(), relational_engine_scratchpad::MemoryError>(())
//! ```
pub mod mem;
pub mod string;
pub mod ffi;
pub use mem::{Memory, MemoryError};

/// Constructor convenience; Rust does not support associated `new!` macros.
#[macro_export]
macro_rules! memory_new {
    () => { $crate::Memory::new() };
}

/// Borrow UTF-8 bytes without allocating or constructing a character array.
#[macro_export]
macro_rules! bytes {
    ($text:expr) => { ($text).as_bytes() };
}
