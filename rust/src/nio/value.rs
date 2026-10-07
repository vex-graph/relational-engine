//! Internal storage kind, separate from ecosystem type IDs.
use std::sync::atomic::AtomicU8;
use crate::primitives::atomic_string::AtomicString;

pub(super) enum Value {
    Bytes(Box<[u8]>),
    AtomicByte(AtomicU8),
    AtomicString(AtomicString),
}
