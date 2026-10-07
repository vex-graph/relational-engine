use relational_engine_scratchpad::{bytes, Memory, MemoryError, string, ffi::*};

#[test]
fn memory_lifetime_growth_and_rejection() {
    let mut memory: Memory = Memory!();
    assert!(memory.is_empty());
    assert!(Memory::default().is_empty());
    let id = memory.copy_bytes(bytes!("hello 🌍")).unwrap();
    for _ in 0..1024 { memory.copy_bytes(b"other").unwrap(); }
    assert_eq!(memory.get(id).unwrap(), "hello 🌍".as_bytes());
    assert_eq!(memory.release(0), Err(MemoryError::UnknownHandle));
    assert_eq!(memory.len(), 1025);
    memory.release(id).unwrap();
    assert_eq!(memory.get(id), Err(MemoryError::UnknownHandle));
    assert_eq!(memory.release(id), Err(MemoryError::UnknownHandle));
    memory.clear();
    let next = memory.copy_bytes(b"").unwrap();
    assert!(next > id);
    assert_eq!(memory.get(next).unwrap(), b"");
    let binary = memory.copy_bytes(b"a\0b").unwrap();
    assert_eq!(memory.get(binary).unwrap(), b"a\0b");
    let mut independent = Memory::new();
    independent.copy_bytes(b"independent").unwrap();
    drop(memory);
    assert_eq!(independent.len(), 1);
}

#[test]
fn byte_text_contract() {
    assert_eq!(string::to_byte_array("hello"), bytes!("hello"));
    assert_eq!(string::as_text(bytes!("🌍")).unwrap(), "🌍");
    assert_eq!(string::as_text(b"").unwrap(), "");
    assert!(string::as_text(&[0xff]).is_err());
}

#[test]
fn ffi_rejection_preserves_outputs_and_recovers() {
    unsafe {
        re_memory_drop(std::ptr::null_mut());
        let owner = re_memory_new();
        let mut id = 99;
        assert_eq!(re_memory_copy(std::ptr::null_mut(), b"x".as_ptr(), 1, &mut id), 1);
        assert_eq!(re_memory_copy(owner, b"x".as_ptr(), 1, std::ptr::null_mut()), 1);
        assert_eq!(re_memory_copy(owner, b"x".as_ptr(), usize::MAX, &mut id), 1);
        assert_eq!(re_memory_copy(owner, std::ptr::null(), 1, &mut id), 1);
        assert_eq!(id, 99);
        assert_eq!(re_memory_copy(owner, b"hello".as_ptr(), 5, &mut id), 0);
        let mut dest = [0xaa; 5];
        let mut length = 99;
        assert_eq!(re_memory_read(std::ptr::null(), id, dest.as_mut_ptr(), 5, &mut length), 1);
        assert_eq!(re_memory_read(owner, id, dest.as_mut_ptr(), 5, std::ptr::null_mut()), 1);
        assert_eq!(re_memory_read(owner, id, dest.as_mut_ptr(), usize::MAX, &mut length), 1);
        assert_eq!(re_memory_read(owner, id, dest.as_mut_ptr(), 4, &mut length), 4);
        assert_eq!(dest, [0xaa; 5]);
        assert_eq!(length, 99);
        assert_eq!(re_memory_read(owner, 0, dest.as_mut_ptr(), 5, &mut length), 3);
        assert_eq!(re_memory_read(owner, id, std::ptr::null_mut(), 5, &mut length), 1);
        assert_eq!(re_memory_read(owner, id, dest.as_mut_ptr(), 5, &mut length), 0);
        assert_eq!(&dest, b"hello");
        assert_eq!(length, 5);
        assert_eq!(re_memory_copy(owner, std::ptr::null(), 0, &mut id), 0);
        assert_eq!(re_memory_read(owner, id, std::ptr::null_mut(), 0, &mut length), 0);
        assert_eq!(length, 0);
        re_memory_drop(owner);
    }
}
