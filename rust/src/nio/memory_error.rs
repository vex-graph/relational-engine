//! Error results for managed storage. Each error preserves stored content.
#[derive(Debug, PartialEq, Eq)]
pub enum MemoryError {
    Allocation,
    Exhausted,
    UnknownHandle,
    WrongKind,
    Bounds,
    Busy,
    Capacity,
}
