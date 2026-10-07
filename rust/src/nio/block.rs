//! Private allocation row: owner-local handle plus its stored value.
use super::value::Value;

pub(super) struct Block {
    pub(super) id: u64,
    pub(super) value: Value,
}
