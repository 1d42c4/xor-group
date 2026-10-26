# xorfiles

A small command-line tool that XORs two files together, byte by byte, and writes the result to a third file.

```
xorfiles [OPTIONS] <FILE_A> <FILE_B> <OUTPUT>
```

Byte *n* of `OUTPUT` is `FILE_A[n] ^ FILE_B[n]`. It runs on Linux and Windows, is written in Rust (2024 edition), and has no dependencies beyond the standard library.

## What is XOR?

XOR ("exclusive or") compares two bits and gives `1` when they differ and `0` when they match. Applied to whole bytes, it works bit by bit:

```
  1111 1111   (0xFF)
^ 0000 1111   (0x0F)
-----------
  1111 0000   (0xF0)
```

XOR is its own inverse, which is what makes it useful for masking data:

```
A ^ B = C        and        C ^ B = A
```

If you XOR a file with a second file, you can get the original back by XORing the result with that same second file again.

## Building

You need a Rust toolchain (1.88 or newer). Install one from <https://rustup.rs>.

```
cargo build --release
```

The binary is created at:

- Linux: `target/release/xorfiles`
- Windows: `target\release\xorfiles.exe`

To install it onto your `PATH` instead:

```
cargo install --path .
```

To build a Windows binary from Linux, add a Windows target (for example `rustup target add x86_64-pc-windows-gnu`, which also needs mingw-w64 installed) and build with `--target`. Otherwise, just build natively on each OS.

## Usage

```
xorfiles [OPTIONS] <FILE_A> <FILE_B> <OUTPUT>
```

| Option | Description |
| --- | --- |
| *(none)* | The two input files must be exactly the same length, otherwise the program fails. |
| `-t`, `--truncate` | Stop at the end of the shorter input file. |
| `-r`, `--repeat` | Repeat `FILE_B` over and over as a key until all of `FILE_A` is covered. |
| `-h`, `--help` | Print usage information. |

`--truncate` and `--repeat` cannot be combined.

On success the program prints how many bytes it wrote, for example `wrote 1048576 bytes to out.bin`.

### Length handling

The three modes differ only in what happens when the inputs are not the same size:

| Mode | `FILE_A` longer | `FILE_B` longer | Output length |
| --- | --- | --- | --- |
| default | error | error | length of both files |
| `--truncate` | extra bytes ignored | extra bytes ignored | length of the shorter file |
| `--repeat` | `FILE_B` is cycled | extra bytes are only used if reached | length of `FILE_A` |

In `--repeat` mode, `FILE_B` is read fully into memory so it can be cycled quickly. That is fine for key-sized files, but avoid it for multi-gigabyte keys. `FILE_B` must not be empty unless `FILE_A` is also empty.

## Examples

XOR two files of equal size:

```
xorfiles a.bin b.bin out.bin
```

A tiny worked example on Linux or macOS:

```
printf '\xff\x0f\xaa' > a.bin
printf '\x0f\x0f\x55' > b.bin
xorfiles a.bin b.bin out.bin
xxd out.bin        # f0 00 ff
```

XOR only as many bytes as the shorter file has:

```
xorfiles --truncate big.bin small.bin out.bin
```

Mask a file with a short key, then restore it by applying the same key again:

```
xorfiles --repeat message.txt key.bin message.xor
xorfiles --repeat message.xor key.bin message.restored.txt
```

On Windows (PowerShell) the commands are the same:

```
.\xorfiles.exe --repeat message.txt key.bin message.xor
```

## Behavior and safety notes

- **Large files are fine.** Inputs are streamed in 64 KiB chunks, so memory use stays small regardless of file size (except for the key in `--repeat` mode).
- **Existing output is overwritten** without a prompt.
- **Output can't be an input.** The program refuses to run if `OUTPUT` is the same file as either input, because creating the output would wipe the input before it was read. (This check compares resolved paths, so it does not detect separate hard links to the same file.)
- **No partial results.** If an error happens partway through, the half-written output file is deleted. Only regular files are removed, so output to something like `/dev/null` is never deleted.
- **Fast failure.** In the default mode, if both inputs are regular files of different sizes, the program fails before creating the output file.
- **Exit codes:** `0` on success, `1` on a runtime error (missing file, length mismatch, I/O error), and `2` on bad command-line usage.

> **Not encryption.** XOR with a repeating key (`--repeat`) is trivially breakable and should not be used to protect sensitive data. XOR is only secure as a one-time pad, meaning a truly random key that is as long as the data and never reused.

## Project layout

```
xorfiles/
├── Cargo.toml     package metadata (edition 2024, Rust 1.88+)
├── README.md
└── src/
    └── main.rs    argument parsing, XOR logic, and unit tests
```

Inside `main.rs`:

- `parse_args` reads the command line by hand, so no argument-parsing crate is needed.
- `xor_streams` handles the default and `--truncate` modes by reading a chunk from each file, XORing them in place, and writing the chunk out.
- `xor_repeating` handles `--repeat` by cycling the key in memory over each chunk of `FILE_A`.
- `run` ties it together: it checks the paths, opens the files, creates the output, and cleans up on failure.

## Tests

```
cargo test
```

The unit tests cover equal-length XOR, mismatch handling in each mode, key repetition, an empty key, and inputs that span several 64 KiB buffers.
