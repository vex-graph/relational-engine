//! Explicit foreign interfaces, separate from native storage behavior.
pub mod memory;
// Preserve the original ffi::re_memory_* client paths.
pub use memory::{re_memory_copy, re_memory_drop, re_memory_new, re_memory_read};
pub use memory::{re_memory_get_byte, re_memory_new_atomic_byte, re_memory_get_atomic_byte,
    re_memory_set_atomic_byte, re_memory_new_atomic_string, re_memory_get_atomic_string,
    re_memory_set_atomic_string};
