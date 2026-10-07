//! Learning storage layer, not a replacement for Vexspoke's slab allocator.
//!
//! ```
//! use relational_engine_scratchpad::{Memory, bytes};
//! let mut memory = Memory!();
//! let hello = memory.copy_bytes(bytes!("hello"))?;
//! assert_eq!(memory.get(hello)?, b"hello");
//! # Ok::<(), relational_engine_scratchpad::MemoryError>(())
//! ```
pub mod nio;
pub mod text;
pub mod ffi;
// Preserve existing short client paths while exposing organized modules.
pub use nio::mem;
pub use text::string;
pub use nio::mem::{Memory, MemoryError};

/// Canonical constructor convenience, sharing the type's name in the macro namespace.
#[macro_export]
macro_rules! Memory {
    () => { $crate::Memory::new() };
}

/// Borrow UTF-8 bytes without allocating or constructing a character array.
#[macro_export]
macro_rules! bytes {
    ($text:expr) => { ($text).as_bytes() };
}
