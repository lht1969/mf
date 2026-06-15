# mf — Make File

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/user/mf)
[![Version](https://img.shields.io/badge/version-1.0.0-blue)](https://crates.io/crates/mf)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange)](rust-toolchain.toml)

**mf** is a cross-platform CLI tool for creating files from the command line — like `md` for files. It supports clipboard content, encoding detection, content type detection, batch creation, and interactive conflict resolution.

## Quick Install

```bash
cargo install mf
```

Or build from source:

```bash
git clone https://github.com/user/mf.git
cd mf
cargo build --release
```

## Quick Usage

```bash
# Create an empty file
mf config.ini

# Create from clipboard
mf output.txt -c

# Batch create multiple files
mf a.txt b.txt c.txt

# Pipe content from stdin
echo "Hello, World!" | mf hello.txt

# Create with specific encoding
mf script.bat --encoding gbk

# Force overwrite existing file
mf report.md -f

# Create then open in default editor
mf notes.md -o
```

## Features

- **Create empty files** — `mf <path>` works like `touch` with extra smarts
- **Clipboard support** — `mf <path> -c` reads the system clipboard
- **Batch creation** — create any number of files in one command
- **Encoding control** — automatic per-extension defaults, manual override via `--encoding`, auto-detection via `chardetng`
- **Content type detection** — auto-detects JSON, XML, HTML, Python, JavaScript, SQL, and plain text; warns on extension mismatch
- **Conflict resolution** — interactive prompts for overwrite/skip/append/rename, plus `--force` and `--no-clobber`
- **Pipe support** — `echo content | mf <path>` reads from stdin
- **Configuration** — TOML config files at project and user level
- **Atomic writes** — writes to a temp file first, then renames; no data loss on failure
- **Cross-platform** — Windows, Linux, macOS

## Configuration

mf supports TOML configuration files with three levels of priority:

1. **Project-level**: `.mf/config.toml` in the current directory
2. **User-level**: `~/.mfconfig` (`%USERPROFILE%\.mfconfig` on Windows)
3. **Built-in defaults**

See the [full usage guide](docs/usage.md#configuration) for details.

## License

MIT
