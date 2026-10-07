//! DEFINITION: ChunkedList grows a flat directory of never-moved row allocations.
//! Each Chunk owns its rows. Directory growth may move headers, not payloads.
//! This first version is append-only and exclusively mutated, not Vexspoke's
//! concurrently published radix tree. References obey Rust borrowing; raw row
//! addresses remain valid through growth until drop, never across destruction.
//! OVERVIEW: ChunkedList<T> { chunks: Vec<Chunk<T>>, rows_per_chunk: usize,
//! len: usize }. API: new/zero/add/get/get_mut/get_chunk/get_chunk_count/
//! get_rows_per_chunk/len/is_empty; bounded to_string/to_string_struct.
//! Geometry rejects zero/ZST/overflow even for empty lists. Allocation failure
//! preserves initialized rows and len (private directory capacity may increase).
//! Rejected add drops the supplied value. No removal, reuse, arbitrary address
//! validation, lock-free publication, whole-list contiguous slice or GPU storage.
use crate::nio::{chunk::{Chunk, CHUNK_ROWS_DEFAULT}, storage_error::StorageError, projection};

pub struct ChunkedList<T> {
    chunks: Vec<Chunk<T>>,
    rows_per_chunk: usize,
    len: usize,
}

impl<T> ChunkedList<T> {
    pub fn new(rows_per_chunk: usize) -> Result<Self, StorageError> {
        let bytes = rows_per_chunk.checked_mul(std::mem::size_of::<T>()).ok_or(StorageError::Layout)?;
        if rows_per_chunk == 0 || std::mem::size_of::<T>() == 0 || bytes > isize::MAX as usize {
            return Err(StorageError::Layout);
        }
        Ok(Self { chunks: Vec::new(), rows_per_chunk, len: 0 })
    }

    pub fn zero() -> Result<Self, StorageError> { Self::new(CHUNK_ROWS_DEFAULT) }

    pub fn add(&mut self, value: T) -> Result<usize, StorageError> {
        let next_len = self.len.checked_add(1).ok_or(StorageError::Capacity)?;
        let chunk_index = self.len / self.rows_per_chunk;
        if chunk_index == self.chunks.len() {
            self.chunks.try_reserve(1).map_err(|_| StorageError::Allocation)?;
            let chunk = Chunk::new(self.rows_per_chunk)?;
            self.chunks.push(chunk);
        }
        self.chunks[chunk_index].add(value)?;
        let index = self.len;
        self.len = next_len;
        Ok(index)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len { return None; }
        self.chunks.get(index / self.rows_per_chunk)?.get(index % self.rows_per_chunk)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len { return None; }
        self.chunks.get_mut(index / self.rows_per_chunk)?.get_mut(index % self.rows_per_chunk)
    }

    pub fn get_chunk(&self, index: usize) -> Option<&[T]> { self.chunks.get(index).map(Chunk::as_slice) }
    pub fn get_chunk_count(&self) -> usize { self.chunks.len() }
    pub fn get_rows_per_chunk(&self) -> usize { self.rows_per_chunk }
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn to_string(&self, dest: &mut [u8], out_truncated: &mut bool) -> bool {
        projection::write(format!("ChunkedList(len={}, chunks={})", self.len, self.chunks.len()), dest, out_truncated)
    }

    pub fn to_string_struct(&self, dest: &mut [u8], out_truncated: &mut bool) -> bool {
        projection::write(format!("ChunkedList {{ chunks: [{} owned], rows_per_chunk: {}, len: {} }}", self.chunks.len(), self.rows_per_chunk, self.len), dest, out_truncated)
    }
}

#[macro_export]
macro_rules! ChunkedList {
    ($type:ty) => { $crate::r#struct::chunked_list::ChunkedList::<$type>::zero() };
    ($type:ty, $rows:expr) => { $crate::r#struct::chunked_list::ChunkedList::<$type>::new($rows) };
}
