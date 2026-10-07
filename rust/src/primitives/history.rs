//! Private primitive string retention ledger. Snapshots live until destruction.
use super::snapshot::Snapshot;

pub(super) struct History {
    pub(super) snapshots: Vec<Box<Snapshot>>,
    pub(super) retained: usize,
}
