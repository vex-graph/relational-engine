//! DEFINITION: TypedChunk owns aligned, fixed-stride object storage and a packed
//! occupancy bitmap. Removing a value transfers ownership out; holes are reused
//! without moving survivors. Only occupied slots are read or dropped. All mutation
//! requires exclusive access; raw pointers expire on removal or owner destruction.
//! OVERVIEW: fields in order: rows Vec<MaybeUninit<T>> (fixed allocation),
//! occupied Vec<u64> (one bit per row), generations Vec<u32> (one per row), len usize.
//! Public: new/zero/add/remove/get/get_mut/add_handle/get_handle/get_handle_mut/
//! remove_handle/generation_at/len/is_empty/get_capacity/is_full, to_string/
//! to_string_struct. Private: validate_geometry, bump_generation; Drop destroys live rows.
//! Invalid geometry and fallible allocations reject before publishing state.
//! Full add drops the incoming value and preserves existing rows. Raw indices are
//! reusable locations; the generation-tagged `Handle` surface (`add_handle` /
//! `get_handle` / `get_handle_mut` / `remove_handle`) is the identity API, so a
//! handle to a removed-and-reused slot is rejected as stale.
use super::handle::Handle;
use super::{projection, storage_error::StorageError};
use std::mem::MaybeUninit;

/// Caller-overridable default chunk geometry, never a pool-wide ceiling.
pub const TYPED_CHUNK_ROWS_DEFAULT: usize = 1024;
// A bitmap word has exactly this many bits by its u64 representation.
const BITMAP_WORD_BITS: usize = u64::BITS as usize;

pub struct TypedChunk<T> {
    rows: Vec<MaybeUninit<T>>,  // fixed allocation of `capacity` slot rows, uninit until written
    occupied: Vec<u64>,         // packed occupancy bitmap: one bit per row slot
    generations: Vec<u32>,      // one generation per row slot; bumped on removal so stale handles reject
    len: usize,                 // number of occupied (live) slots
}

impl<T> TypedChunk<T> {
    /// Validate nonzero, non-ZST row geometry and ensure its byte size is representable.
    pub(crate) fn validate_geometry(capacity: usize) -> Result<(), StorageError> {
        let bytes = capacity.checked_mul(std::mem::size_of::<T>())
            .ok_or(StorageError::Layout)?;
        if capacity == 0 || std::mem::size_of::<T>() == 0 || bytes > isize::MAX as usize {
            return Err(StorageError::Layout);
        }
        Ok(())
    }

    /// Allocate fixed typed-row slots and a zeroed occupancy bitmap for `capacity` rows.
    pub fn new(capacity: usize) -> Result<Self, StorageError> {
        Self::validate_geometry(capacity)?;
        let words = capacity.div_ceil(BITMAP_WORD_BITS);
        let mut rows = Vec::new();
        rows.try_reserve_exact(capacity).map_err(|_| StorageError::Allocation)?;
        rows.resize_with(capacity, MaybeUninit::uninit);
        let mut occupied = Vec::new();
        occupied.try_reserve_exact(words).map_err(|_| StorageError::Allocation)?;
        occupied.resize(words, 0);
        let mut generations = Vec::new();
        generations.try_reserve_exact(capacity).map_err(|_| StorageError::Allocation)?;
        generations.resize(capacity, 0u32);
        Ok(Self { rows, occupied, generations, len: 0 })
    }

    /// Create a typed chunk using the named default row capacity.
    pub fn zero() -> Result<Self, StorageError> { Self::new(TYPED_CHUNK_ROWS_DEFAULT) }

    /// Insert into the first free slot and return its reusable pool-local index.
    pub fn add(&mut self, value: T) -> Result<usize, StorageError> {
        if self.is_full() { return Err(StorageError::Capacity); }
        for (word_index, word) in self.occupied.iter_mut().enumerate() {
            let bit = (!*word).trailing_zeros() as usize;
            let index = word_index * BITMAP_WORD_BITS + bit;
            if bit < BITMAP_WORD_BITS && index < self.rows.len() {
                self.rows[index].write(value);
                *word |= 1u64 << bit;
                self.len += 1;
                return Ok(index);
            }
        }
        // The private live count and bitmap agree; no public path can reach this.
        Err(StorageError::Capacity)
    }

    /// Remove an occupied slot and transfer its value to the caller; reject empty or invalid indices.
    pub fn remove(&mut self, index: usize) -> Result<T, StorageError> {
        if self.get(index).is_none() { return Err(StorageError::Bounds); }
        self.occupied[index / BITMAP_WORD_BITS] &= !(1u64 << (index % BITMAP_WORD_BITS));
        self.bump_generation(index);
        self.len -= 1;
        // SAFETY: occupancy proved initialization; exclusive access transfers the
        // value exactly once and clearing its bit prevents later reads/drops.
        Ok(unsafe { self.rows[index].assume_init_read() })
    }

    /// Borrow a live value at `index`; empty slots and out-of-range indices return `None`.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.rows.len() || self.occupied[index / BITMAP_WORD_BITS] &
            (1u64 << (index % BITMAP_WORD_BITS)) == 0 { return None; }
        // SAFETY: this bit is set only after writing an initialized T.
        Some(unsafe { self.rows[index].assume_init_ref() })
    }

    /// Mutably borrow a live value at `index`; empty slots and out-of-range indices return `None`.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if self.get(index).is_none() { return None; }
        // SAFETY: initialization checked above; &mut self excludes all borrowers.
        Some(unsafe { self.rows[index].assume_init_mut() })
    }

    /// Return the number of occupied slots.
    pub fn len(&self) -> usize { self.len }
    /// Return whether no slots are occupied.
    pub fn is_empty(&self) -> bool { self.len == 0 }
    /// Return the fixed number of row slots.
    pub fn get_capacity(&self) -> usize { self.rows.len() }
    /// Return whether every row slot is occupied.
    pub fn is_full(&self) -> bool { self.len == self.rows.len() }

    /// The generation stored for a row slot (0 while the slot was never freed).
    pub fn generation_at(&self, index: usize) -> u32 {
        self.generations.get(index).copied().unwrap_or(0)
    }

    // Advance a freed slot's generation so every earlier handle to it goes stale.
    // Generation 0 is reserved for a never-freed slot, so wrap skips back to 1.
    fn bump_generation(&mut self, index: usize) {
        let next = self.generations[index].wrapping_add(1);
        self.generations[index] = if next == 0 { 1 } else { next };
    }

    /// Insert and return a generation-tagged identity for the new value.
    pub fn add_handle(&mut self, value: T) -> Result<Handle, StorageError> {
        let index = self.add(value)?;
        Ok(Handle::new(index, self.generations[index]))
    }

    /// Borrow the value a live handle names; a stale or out-of-range handle returns `None`.
    pub fn get_handle(&self, handle: Handle) -> Option<&T> {
        let index = handle.index();
        if index >= self.rows.len() || self.generations[index] != handle.generation() {
            return None;
        }
        self.get(index)
    }

    /// Mutably borrow the value a live handle names; a stale handle returns `None`.
    pub fn get_handle_mut(&mut self, handle: Handle) -> Option<&mut T> {
        let index = handle.index();
        if index >= self.rows.len() || self.generations[index] != handle.generation() {
            return None;
        }
        self.get_mut(index)
    }

    /// Remove the value a live handle names and invalidate the handle; a stale handle rejects.
    pub fn remove_handle(&mut self, handle: Handle) -> Result<T, StorageError> {
        let index = handle.index();
        if index >= self.rows.len() || self.generations[index] != handle.generation() {
            return Err(StorageError::Bounds);
        }
        self.remove(index)
    }

    /// Write a bounded value summary and report whether the destination was truncated.
    pub fn to_string(&self, dest: &mut [u8], out_truncated: &mut bool) -> bool {
        projection::write(format!("TypedChunk(len={}, capacity={})", self.len, self.rows.len()), dest, out_truncated)
    }

    /// Write a bounded one-level field summary and report destination truncation.
    pub fn to_string_struct(&self, dest: &mut [u8], out_truncated: &mut bool) -> bool {
        projection::write(format!("TypedChunk {{ rows: [{} slots], occupied: [{} words], len: {} }}",
            self.rows.len(), self.occupied.len(), self.len), dest, out_truncated)
    }
}

impl<T> Drop for TypedChunk<T> {
    /// Drop only occupied values; uninitialized slots are left untouched.
    fn drop(&mut self) {
        for index in 0..self.rows.len() {
            if self.get(index).is_some() {
                // SAFETY: only initialized, still-owned slots are dropped.
                unsafe { self.rows[index].assume_init_drop(); }
            }
        }
    }
}

#[macro_export]
macro_rules! TypedChunk {
    ($type:ty) => { $crate::nio::typed_chunk::TypedChunk::<$type>::zero() };
    ($type:ty, $capacity:expr) => { $crate::nio::typed_chunk::TypedChunk::<$type>::new($capacity) };
}
