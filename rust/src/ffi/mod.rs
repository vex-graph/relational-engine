//! Explicit foreign interfaces, separate from native storage behavior.
pub mod memory;
// Preserve the original ffi::re_memory_* client paths.
pub use memory::{re_memory_copy, re_memory_drop, re_memory_new, re_memory_read};
