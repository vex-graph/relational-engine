"""Registered Rust owner suite and actual C static-library ABI client."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]

def run(*args, env=None):
    subprocess.run(args, cwd=ROOT, env=env, check=True, timeout=90)

with tempfile.TemporaryDirectory(prefix="relational-owner-", dir=os.environ.get("TMPDIR")) as tmp:
    target = Path(tmp)
    env = dict(os.environ, CARGO_TARGET_DIR=tmp, RUSTFLAGS="-D warnings")
    run("cargo", "test", "--offline", "--locked", "--manifest-path", "rust/Cargo.toml", env=env)
    run("cargo", "test", "--release", "--offline", "--locked", "--manifest-path", "rust/Cargo.toml", env=env)
    run("cargo", "build", "--offline", "--locked", "--manifest-path", "rust/Cargo.toml", env=env)
    binary = str(target / "ffi-test")
    run("clang", "-std=c23", "-Wall", "-Wextra", "-Werror", "-Irust/include",
        "tests/rust/ffi_test.c", str(target / "debug/librelational_engine_scratchpad.a"),
        "-o", binary)
    run(binary)
    result = subprocess.run([
        "rustc", "--edition=2024", "tests/rust/borrow_rejection.rs",
        "--extern", f"relational_engine_scratchpad={target}/debug/librelational_engine_scratchpad.rlib",
        "-L", f"dependency={target}/debug/deps", "-o", str(target / "borrow-rejection")],
        cwd=ROOT, capture_output=True, text=True, timeout=90)
    assert result.returncode != 0 and "E0502" in result.stderr, result.stderr
    run("clang", "-std=c23", "-Wall", "-Wextra", "-Werror", "-fsanitize=address,undefined",
        "-Irust/include", "tests/rust/ffi_test.c",
        str(target / "debug/librelational_engine_scratchpad.a"), "-o", binary)
    run(binary)
    print("PASS: debug/release owners, executable docs, C ABI, C ASan/UBSan, E0502 borrow rejection")
