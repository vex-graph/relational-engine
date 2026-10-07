//! Private immutable primitive string version; never mutated after publication.
pub(super) struct Snapshot {
    pub(super) bytes: Box<[u8]>,
}
