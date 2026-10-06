"""Standalone owner check for scaffold metadata, not imported engine behavior."""
from pathlib import Path
import json
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True,
                          capture_output=True, timeout=90, **kwargs)


def main():
    parts = ("nio", "io", "reflection", "relational")
    for part in parts:
        assert (ROOT / "src" / part).is_dir()
        assert not (ROOT / part).exists()
    sources = sorted((ROOT / "src").rglob("*.c"))
    assert sources
    ignored = ["rust/target/generated.o", "build/generated.o", "cmake-build-debug/CMakeCache.txt",
               ".idea/workspace.xml", "generated.dll", "generated.dSYM/Contents/data"]
    tracked = ["src/io/file.c", "rust/src/lib.rs", "rust/Cargo.toml", "rust/Cargo.lock",
               "tests/scaffold_test.py"]
    result = run("git", "check-ignore", "--no-index", "--stdin",
                 input="\n".join(ignored + tracked) + "\n")
    assert set(result.stdout.splitlines()) == set(ignored)
    assert "IDE metadata" in (ROOT / "README.md").read_text()
    assert "No allocator" in (ROOT / "README.md").read_text()
    assert "contains no port" in (ROOT / "rust/README.md").read_text()
    assert "https://gist.github.com/vex-graph/" in (ROOT / "CONTRIBUTING.md").read_text()
    assert (ROOT / "src/LICENSE").read_text().startswith("Boost Software License")
    assert (ROOT / "LICENSE").read_text().startswith("MIT License")
    assert "EXCLUDE_FROM_ALL" in (ROOT / "CMakeLists.txt").read_text()
    scratch = os.environ.get("TMPDIR")
    with tempfile.TemporaryDirectory(prefix="relational-scaffold-", dir=scratch) as tmp:
        tmp = Path(tmp)
        run("cmake", "-S", str(ROOT), "-B", str(tmp / "ide"))
        commands = json.loads((tmp / "ide/compile_commands.json").read_text())
        assert {Path(entry["file"]).resolve() for entry in commands} == set(sources)
        for entry in commands:
            command = entry["command"]
            assert all(flag in command for flag in ("-Wall", "-Wextra", "-Werror", "-std=gnu"))
        env = dict(os.environ, CARGO_TARGET_DIR=str(tmp / "cargo"), RUSTFLAGS="-D warnings")
        run("cargo", "check", "--offline", "--locked", "--manifest-path", "rust/Cargo.toml", env=env)
    print(f"PASS: {len(sources)} C code-model entries, mixed ignores, local layout and Rust scaffold check")


if __name__ == "__main__":
    main()
