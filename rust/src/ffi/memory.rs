//! DEFINITION: ffi/memory is the opaque C bridge for the learning backend.
//! OVERVIEW: re_memory_new/drop/copy/read. Status 0=success, 1=invalid,
//! 2=allocation/identity exhaustion, 3=unknown handle, 4=capacity too small.
//! Non-null pointers must be valid, aligned, live and externally synchronized.
//! Inputs/outputs must not alias the owner. read uses memmove semantics.
//! Handles are owner-local; using an ID with another owner is caller misuse.
//! No borrowed block pointer crosses FFI. Errors preserve outputs and content.
//! Null checks cannot establish arbitrary pointer validity. No Vexspoke ABI parity.
use crate::{Memory, MemoryError};

#[unsafe(no_mangle)]
pub extern "C" fn re_memory_new() -> *mut Memory {
    // Box allocation follows Rust's process-level OOM policy (may abort).
    Box::into_raw(Box::new(Memory::new()))
}

/// # Safety
/// Owner is null or a live result of re_memory_new, released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn re_memory_drop(owner: *mut Memory) {
    if !owner.is_null() { drop(unsafe { Box::from_raw(owner) }); }
}

/// # Safety
/// Owner is live/exclusive; source spans length readable bytes (or null at zero).
/// Output points to writable u64 storage, disjoint from owner/source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn re_memory_copy(
    owner: *mut Memory, source: *const u8, length: usize, output: *mut u64,
) -> u32 {
    if owner.is_null() || output.is_null() || (source.is_null() && length != 0)
        || length > isize::MAX as usize { return 1; }
    let bytes = if length == 0 { &[] } else {
        unsafe { std::slice::from_raw_parts(source, length) }
    };
    match unsafe { &mut *owner }.copy_bytes(bytes) {
        Ok(id) => { unsafe { *output = id }; 0 }
        Err(MemoryError::UnknownHandle) => 3,
        Err(_) => 2,
    }
}

/// # Safety
/// Owner is live/shared; destination spans capacity writable bytes. Output is
/// writable size_t storage disjoint from destination and owner. No concurrent mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn re_memory_read(
    owner: *const Memory, id: u64, destination: *mut u8, capacity: usize,
    output_length: *mut usize,
) -> u32 {
    if owner.is_null() || output_length.is_null() || capacity > isize::MAX as usize {
        return 1;
    }
    let bytes = match unsafe { &*owner }.get(id) { Ok(bytes) => bytes, Err(_) => return 3 };
    if capacity < bytes.len() { return 4; }
    if !bytes.is_empty() {
        if destination.is_null() { return 1; }
        unsafe { std::ptr::copy(bytes.as_ptr(), destination, bytes.len()) };
    }
    unsafe { *output_length = bytes.len() };
    0
}
