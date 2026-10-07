//! DEFINITION: nio/mem owns byte blocks; C does not own their allocations.
//! This cold, single-owner teaching backend uses the Rust heap, not slabs.
//! OVERVIEW: Memory { blocks: Vec<Block>, next_id: u64 }.
//! Private Block { id: u64, bytes: Box<[u8]> }; identity is never reused.
//! API: new, copy_bytes, get, release, clear, len, is_empty.
//! Rejection preserves state. Rust borrows prevent release during a read.

#[derive(Debug, PartialEq, Eq)]
pub enum MemoryError { Allocation, Exhausted, UnknownHandle }
struct Block { id: u64, bytes: Box<[u8]> }
pub struct Memory { blocks: Vec<Block>, next_id: u64 }

impl Default for Memory {
    fn default() -> Self { Self::new() }
}

impl Memory {
    pub fn new() -> Self { Self { blocks: Vec::new(), next_id: 1 } }

    /// Copy bytes, including embedded NUL. Empty blocks are valid.
    pub fn copy_bytes(&mut self, source: &[u8]) -> Result<u64, MemoryError> {
        let next = self.next_id.checked_add(1).ok_or(MemoryError::Exhausted)?;
        self.blocks.try_reserve(1).map_err(|_| MemoryError::Allocation)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(source.len()).map_err(|_| MemoryError::Allocation)?;
        bytes.extend_from_slice(source);
        let id = self.next_id;
        self.blocks.push(Block { id, bytes: bytes.into_boxed_slice() });
        self.next_id = next;
        Ok(id)
    }

    /// A borrow cannot outlive this owner or coexist with a mutable operation.
    pub fn get(&self, id: u64) -> Result<&[u8], MemoryError> {
        self.blocks.iter().find(|block| block.id == id)
            .map(|block| block.bytes.as_ref()).ok_or(MemoryError::UnknownHandle)
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
