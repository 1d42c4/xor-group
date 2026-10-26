# xor_cli

A simple, zero-dependency Rust CLI that XORs corresponding bytes of two files and writes the result to a third file.

Works on **Linux**, **Windows**, and **macOS**.

## Requirements

- Rust 1.85+ (2024 edition)

## Build

```bash
cargo build --release
```

Binary location:
- Linux / macOS: `target/release/xor_cli`
- Windows: `target\release\xor_cli.exe`

## Usage

```bash
xor_cli <file1> <file2> <output>
```

| Argument | Description                          |
|----------|--------------------------------------|
| `file1`  | First input file                     |
| `file2`  | Second input file                    |
| `output` | Destination file for the XOR result  |

### Behavior

- Streams both files in 8 KiB chunks (low memory usage)
- XORs every pair of corresponding bytes
- Output length = `min(len(file1), len(file2))`
- Overwrites the output file if it already exists
- Exits with a non-zero status and prints an error on failure

## Example

```bash
echo -n "Hello" > a.bin
echo -n "World" > b.bin
./target/release/xor_cli a.bin b.bin out.bin
```

`out.bin` will contain the 5 bytes:

```
1f 0a 1e 00 0b
```

(`H^W`, `e^o`, `l^r`, `l^l`, `o^d`)

## Notes

- Pure standard library — no external crates
- Cross-platform (Linux & Windows)
- Suitable for large files thanks to buffered I/O
