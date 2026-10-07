//! Private immutable string version; never moved or mutated after publication.
pub(super) struct Snapshot {
    pub(super) bytes: Box<[u8]>,
}
