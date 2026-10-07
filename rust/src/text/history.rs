//! Private string retention ledger. Owned snapshots stay alive until destruction.
use super::snapshot::Snapshot;

pub(super) struct History {
    pub(super) snapshots: Vec<Box<Snapshot>>,
    pub(super) retained: usize,
}
