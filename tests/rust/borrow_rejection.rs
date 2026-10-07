// Compile-negative proof: a live byte borrow prevents releasing its owner block.
use relational_engine_scratchpad::Memory;
fn main() {
    let mut memory = Memory::new();
    let id = memory.copy_bytes(b"hello").unwrap();
    let bytes = memory.get(id).unwrap();
    memory.release(id).unwrap();
    assert_eq!(bytes, b"hello");
}
