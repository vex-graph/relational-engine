//! DEFINITION: primitives/string projects bytes without Rust character storage.
//! OVERVIEW: no owned class. to_byte_array borrows; as_text validates UTF-8.
//! Byte storage permits arbitrary bytes; text projection rejects invalid UTF-8.
pub fn to_byte_array(text: &str) -> &[u8] { text.as_bytes() }
pub fn as_text(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    std::str::from_utf8(bytes)
}
