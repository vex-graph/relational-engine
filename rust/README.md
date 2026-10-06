# Rust interpretation

This Cargo package exists for compiler and IDE discovery. It contains no port
yet. Add real, owner-tested translations incrementally; do not fill missing C
dependencies with Rust stubs or claim equivalence from a successful Cargo check.

Rust will manage storage lifetime; C clients may process borrowed spans directly.
The ABI, growth/borrow policy and concurrency model remain design work.
