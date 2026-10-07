//! Planned file reads/writes, buffered readers/writers, gathering, indexing and
//! watching (FFF-style discovery). No runtime file API is implemented here yet.
//! File-backed byte mappings belong to nio, not an implicit read in a getter.
//! Manifest-backed persistence is a future contract: use record IDs/file offsets,
//! not serialized RAM pointers. Durability and concurrent commits remain unproved.
