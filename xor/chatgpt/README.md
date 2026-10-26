# xor-files

A small cross-platform Rust command-line tool for Windows and Linux. It XORs corresponding bytes from two input files and writes the result to a third file.

## Requirements

- Rust toolchain with Rust 2024 edition support
- Cargo

## Build

From the project directory:

```sh
cargo build --release
```

The executable is created at:

- Linux: `target/release/xor-files`
- Windows: `target/release/xor-files.exe`

## Usage

```text
xor-files <input1> <input2> <output>
```

Examples:

Linux:

```sh
./target/release/xor-files file1.bin file2.bin result.bin
```

Windows PowerShell:

```powershell
.\target\release\xor-files.exe file1.bin file2.bin result.bin
```

Use `xor-files --help` for the usage summary.

## Behavior

- Each output byte is `input1[i] ^ input2[i]`.
- Processing stops at the end of the shorter input; any unmatched trailing bytes are ignored.
- Buffered I/O is used, so the complete files do not need to fit in memory.
- No third-party crates are required.
- The program rejects an output path that resolves to the same canonical path as either input, helping prevent accidental truncation of an input file.

Example: `0xAA ^ 0x0F = 0xA5`.

## Note

XOR itself is reversible, but this utility is not a complete or secure encryption system.

## License

MIT
