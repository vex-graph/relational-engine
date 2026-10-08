//! DEFINITION: TypedPool lazily allocates TypedChunks of one Rust type. A flat
//! directory may move, but live object allocations never do. Removal reuses holes;
//! explicit empty-chunk release keeps directory positions so survivors keep indices.
//! OVERVIEW: chunks Vec<Option<TypedChunk<T>>> (stable directory positions),
//! rows_per_chunk usize (immutable geometry), len usize (total live count).
//! Public: new/zero/add/remove/get/get_mut/release_empty_chunks/get_chunk_count/
//! get_rows_per_chunk/len/is_empty/to_string/to_string_struct.
//! Failed admission drops the incoming T, preserves live state and may grow only
//! private directory capacity. Exclusive mutation excludes Rust borrows; callers
//! must also exclude raw-pointer users before remove, release or destruction.
//! Returned usize locations are pool-local and reusable, NOT identities. No
//! registered ecosystem type ID, generation, concurrent allocator or C ABI yet.
use crate::nio::{typed_chunk::{TypedChunk, TYPED_CHUNK_ROWS_DEFAULT}, storage_error::StorageError, projection};

pub struct TypedPool<T> {
    chunks: Vec<Option<TypedChunk<T>>>,
    rows_per_chunk: usize,
    len: usize,
}

impl<T> TypedPool<T> {
    pub fn new(rows_per_chunk: usize) -> Result<Self, StorageError> {
        TypedChunk::<T>::validate_geometry(rows_per_chunk)?;
        Ok(Self { chunks: Vec::new(), rows_per_chunk, len: 0 })
    }

    pub fn zero() -> Result<Self, StorageError> { Self::new(TYPED_CHUNK_ROWS_DEFAULT) }

    pub fn add(&mut self, value: T) -> Result<usize, StorageError> {
        let next_len = self.len.checked_add(1).ok_or(StorageError::Capacity)?;
        // Reuse all allocated holes before allocating another chunk.
        for (chunk_index, entry) in self.chunks.iter_mut().enumerate() {
            if let Some(chunk) = entry {
                if !chunk.is_full() {
                    let index = chunk_index * self.rows_per_chunk + chunk.add(value)?;
                    self.len = next_len;
                    return Ok(index);
                }
            }
        }
        let chunk_index = self.chunks.iter().position(Option::is_none).unwrap_or(self.chunks.len());
        let base = chunk_index.checked_mul(self.rows_per_chunk).ok_or(StorageError::Capacity)?;
        base.checked_add(self.rows_per_chunk - 1).ok_or(StorageError::Capacity)?;
        if chunk_index == self.chunks.len() {
            self.chunks.try_reserve(1).map_err(|_| StorageError::Allocation)?;
        }
        let mut chunk = TypedChunk::new(self.rows_per_chunk)?;
        let index = base + chunk.add(value)?;
        if chunk_index == self.chunks.len() { self.chunks.push(Some(chunk)); }
        else { self.chunks[chunk_index] = Some(chunk); }
        self.len = next_len;
        Ok(index)
    }

    pub fn remove(&mut self, index: usize) -> Result<T, StorageError> {
        let chunk = self.chunks.get_mut(index / self.rows_per_chunk)
            .and_then(Option::as_mut).ok_or(StorageError::Bounds)?;
        let value = chunk.remove(index % self.rows_per_chunk)?;
        self.len -= 1;
        Ok(value)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.chunks.get(index / self.rows_per_chunk)?.as_ref()?.get(index % self.rows_per_chunk)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.chunks.get_mut(index / self.rows_per_chunk)?.as_mut()?.get_mut(index % self.rows_per_chunk)
    }

    /// Release backing allocations only; retain directory positions for survivors.
    pub fn release_empty_chunks(&mut self) -> usize {
        let mut released = 0;
        for entry in &mut self.chunks {
            if entry.as_ref().is_some_and(TypedChunk::is_empty) {
                *entry = None;
                released += 1;
            }
        }
        released
    }

    pub fn get_chunk_count(&self) -> usize { self.chunks.iter().filter(|entry| entry.is_some()).count() }
    pub fn get_rows_per_chunk(&self) -> usize { self.rows_per_chunk }
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn to_string(&self, dest: &mut [u8], out_truncated: &mut bool) -> bool {
        projection::write(format!("TypedPool(len={}, chunks={})", self.len, self.get_chunk_count()), dest, out_truncated)
    }

    pub fn to_string_struct(&self, dest: &mut [u8], out_truncated: &mut bool) -> bool {
        projection::write(format!("TypedPool {{ chunks: [{} directory entries, {} allocated], rows_per_chunk: {}, len: {} }}",
            self.chunks.len(), self.get_chunk_count(), self.rows_per_chunk, self.len), dest, out_truncated)
    }
}

#[macro_export]
macro_rules! TypedPool {
    ($type:ty) => { $crate::r#struct::typed_pool::TypedPool::<$type>::zero() };
    ($type:ty, $rows:expr) => { $crate::r#struct::typed_pool::TypedPool::<$type>::new($rows) };
}
