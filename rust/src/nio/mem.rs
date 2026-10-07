//! DEFINITION: nio/mem owns byte blocks; C does not own their allocations.
//! This cold, single-owner teaching backend uses the Rust heap, not slabs.
//! OVERVIEW: Memory { blocks: Vec<Block>, next_id: u64 }.
//! Block and Value are private module types in block.rs and value.rs.
//! API: new/copy_bytes/get/get_byte; new/get/set_atomic_byte;
//! new/get/set_atomic_string; release/clear/len/is_empty.
//! Registration/release/clear need exclusive ownership; atomic get/set share &self.
//! Rejection preserves content. Kind tags are internal, NOT ecosystem type IDs.
//! Atomic Bytes publish with release/acquire. Strings retain immutable snapshots.
use std::sync::atomic::{AtomicU8, Ordering};
use crate::text::atomic_string::AtomicString;
pub use super::memory_error::MemoryError;
use super::{block::Block, value::Value};

pub struct Memory { blocks: Vec<Block>, next_id: u64 }

impl Default for Memory {
    fn default() -> Self { Self::new() }
}

impl Memory {
    pub fn new() -> Self { Self { blocks: Vec::new(), next_id: 1 } }

    /// Copy Bytes, including embedded NUL. Empty blocks are valid.
    pub fn copy_bytes(&mut self, source: &[u8]) -> Result<u64, MemoryError> {
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(source.len()).map_err(|_| MemoryError::Allocation)?;
        bytes.extend_from_slice(source);
        self.insert(Value::Bytes(bytes.into_boxed_slice()))
    }

    fn insert(&mut self, value: Value) -> Result<u64, MemoryError> {
        let next = self.next_id.checked_add(1).ok_or(MemoryError::Exhausted)?;
        self.blocks.try_reserve(1).map_err(|_| MemoryError::Allocation)?;
        let id = self.next_id;
        self.blocks.push(Block { id, value });
        self.next_id = next;
        Ok(id)
    }

    /// A borrow cannot outlive this owner or coexist with a mutable operation.
    pub fn get(&self, id: u64) -> Result<&[u8], MemoryError> {
        match self.value(id)? {
            Value::Bytes(bytes) => Ok(bytes),
            _ => Err(MemoryError::WrongKind),
        }
    }

    fn value(&self, id: u64) -> Result<&Value, MemoryError> {
        self.blocks.iter().find(|block| block.id == id)
            .map(|block| &block.value).ok_or(MemoryError::UnknownHandle)
    }

    pub fn get_byte(&self, id: u64, index: usize) -> Result<u8, MemoryError> {
        self.get(id)?.get(index).copied().ok_or(MemoryError::Bounds)
    }

    pub fn new_atomic_byte(&mut self, value: u8) -> Result<u64, MemoryError> {
        self.insert(Value::AtomicByte(AtomicU8::new(value)))
    }

    pub fn get_atomic_byte(&self, id: u64) -> Result<u8, MemoryError> {
        match self.value(id)? {
            Value::AtomicByte(value) => Ok(value.load(Ordering::Acquire)),
            _ => Err(MemoryError::WrongKind),
        }
    }

    pub fn set_atomic_byte(&self, id: u64, value: u8) -> Result<(), MemoryError> {
        match self.value(id)? {
            Value::AtomicByte(cell) => { cell.store(value, Ordering::Release); Ok(()) }
            _ => Err(MemoryError::WrongKind),
        }
    }

    pub fn new_atomic_string(&mut self, bytes: &[u8], retention_limit: usize) -> Result<u64, MemoryError> {
        self.insert(Value::AtomicString(AtomicString::new(bytes, retention_limit)?))
    }

    pub fn get_atomic_string(&self, id: u64) -> Result<&[u8], MemoryError> {
        match self.value(id)? {
            Value::AtomicString(value) => Ok(value.get()),
            _ => Err(MemoryError::WrongKind),
        }
    }

    pub fn set_atomic_string(&self, id: u64, bytes: &[u8]) -> Result<(), MemoryError> {
        match self.value(id)? {
            Value::AtomicString(value) => value.set(bytes),
            _ => Err(MemoryError::WrongKind),
        }
    }

    pub fn release(&mut self, id: u64) -> Result<(), MemoryError> {
        let index = self.blocks.iter().position(|block| block.id == id)
            .ok_or(MemoryError::UnknownHandle)?;
        self.blocks.swap_remove(index);
        Ok(())
    }

    /// Invalidate all handles; do not reset the identity counter.
    pub fn clear(&mut self) { self.blocks.clear(); }
    pub fn len(&self) -> usize { self.blocks.len() }
    pub fn is_empty(&self) -> bool { self.blocks.is_empty() }
}
