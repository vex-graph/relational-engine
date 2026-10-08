//! DEFINITION: TypedChunk owns aligned, fixed-stride object storage and a packed
//! occupancy bitmap. Removing a value transfers ownership out; holes are reused
//! without moving survivors. Only occupied slots are read or dropped. All mutation
//! requires exclusive access; raw pointers expire on removal or owner destruction.
//! OVERVIEW: fields in order: rows Vec<MaybeUninit<T>> (fixed allocation),
//! occupied Vec<u64> (one bit per row), len usize (live count).
//! Public: new/zero/add/remove/get/get_mut/len/is_empty/get_capacity/is_full,
//! to_string/to_string_struct. Private: validate_geometry; Drop destroys live rows.
//! Invalid geometry and fallible allocations reject before publishing state.
//! Full add drops the incoming value and preserves existing rows. Indices are
//! reusable locations, NOT generation-tagged identities or stale-reference checks.
use super::{projection, storage_error::StorageError};
use std::mem::MaybeUninit;

/// Caller-overridable default chunk geometry, never a pool-wide ceiling.
pub const TYPED_CHUNK_ROWS_DEFAULT: usize = 1024;
// A bitmap word has exactly this many bits by its u64 representation.
const BITMAP_WORD_BITS: usize = u64::BITS as usize;

pub struct TypedChunk<T> {
    rows: Vec<MaybeUninit<T>>,
    occupied: Vec<u64>,
    len: usize,
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
        Ok(Self { rows, occupied, len: 0 })
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
