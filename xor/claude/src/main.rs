use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const BUF_SIZE: usize = 64 * 1024;

const USAGE: &str = "\
xorfiles - XOR two files byte-by-byte into a third file

USAGE:
    xorfiles [OPTIONS] <FILE_A> <FILE_B> <OUTPUT>

OPTIONS:
    -t, --truncate    Stop at the end of the shorter file
    -r, --repeat      Repeat FILE_B as a key until FILE_A is fully covered
                      (FILE_B is loaded into memory)
    -h, --help        Show this help

By default the two input files must be exactly the same length.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Fail if the inputs differ in length (default).
    Strict,
    /// Stop at the end of the shorter input.
    Truncate,
    /// Cycle the second file as a repeating key over the first.
    Repeat,
}

struct Args {
    mode: Mode,
    a: PathBuf,
    b: PathBuf,
    out: PathBuf,
}

fn set_mode(slot: &mut Option<Mode>, new: Mode) -> Result<(), String> {
    if let Some(old) = *slot
        && old != new
    {
        return Err("--truncate and --repeat cannot be used together".into());
    }
    *slot = Some(new);
    Ok(())
}

/// Returns Ok(None) when help was requested.
fn parse_args() -> Result<Option<Args>, String> {
    let mut mode: Option<Mode> = None;
    let mut paths: Vec<PathBuf> = Vec::new();

    for arg in env::args_os().skip(1) {
        match arg.to_str() {
            Some("-h") | Some("--help") => return Ok(None),
            Some("-t") | Some("--truncate") => set_mode(&mut mode, Mode::Truncate)?,
            Some("-r") | Some("--repeat") => set_mode(&mut mode, Mode::Repeat)?,
            Some(s) if s.starts_with('-') && s.len() > 1 => {
                return Err(format!("unknown option: {s}"));
            }
            _ => paths.push(PathBuf::from(&arg)),
        }
    }

    let [a, b, out]: [PathBuf; 3] = paths
        .try_into()
        .map_err(|p: Vec<PathBuf>| format!("expected 3 file arguments, got {}", p.len()))?;

    Ok(Some(Args {
        mode: mode.unwrap_or(Mode::Strict),
        a,
        b,
        out,
    }))
}

/// Read until `buf` is full or EOF. Returns the number of bytes read;
/// a value smaller than `buf.len()` means EOF was reached.
fn fill<R: Read>(r: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// XOR two streams chunk by chunk. In `Strict` mode a length mismatch is an
/// error; in `Truncate` mode output stops at the end of the shorter stream.
fn xor_streams<A: Read, B: Read, W: Write>(
    a: &mut A,
    b: &mut B,
    out: &mut W,
    mode: Mode,
) -> io::Result<u64> {
    let mut buf_a = vec![0u8; BUF_SIZE];
    let mut buf_b = vec![0u8; BUF_SIZE];
    let mut total: u64 = 0;

    loop {
        let na = fill(a, &mut buf_a)?;
        let nb = fill(b, &mut buf_b)?;
        let n = na.min(nb);

        for (x, y) in buf_a[..n].iter_mut().zip(&buf_b[..n]) {
            *x ^= *y;
        }
        out.write_all(&buf_a[..n])?;
        total += n as u64;

        if na != nb && mode == Mode::Strict {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "input files differ in length; use --truncate or --repeat",
            ));
        }
        if n < BUF_SIZE {
            // At least one input reached EOF.
            break;
        }
    }
    Ok(total)
}

/// XOR `data` against `key`, cycling `key` as often as needed.
fn xor_repeating<A: Read, W: Write>(data: &mut A, key: &[u8], out: &mut W) -> io::Result<u64> {
    let mut buf = vec![0u8; BUF_SIZE];
    let mut total: u64 = 0;
    let mut pos = 0usize;

    loop {
        let n = fill(data, &mut buf)?;
        if n == 0 {
            break;
        }
        if key.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "second file is empty, so it cannot be repeated",
            ));
        }
        for byte in &mut buf[..n] {
            *byte ^= key[pos];
            pos += 1;
            if pos == key.len() {
                pos = 0;
            }
        }
        out.write_all(&buf[..n])?;
        total += n as u64;
    }
    Ok(total)
}

fn same_file(x: &Path, y: &Path) -> bool {
    match (fs::canonicalize(x), fs::canonicalize(y)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false, // e.g. the output doesn't exist yet
    }
}

fn open(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))
}

fn run(args: &Args) -> Result<u64, String> {
    if same_file(&args.out, &args.a) || same_file(&args.out, &args.b) {
        return Err("output file must be different from both input files".into());
    }

    let mut a = open(&args.a)?;
    let mut b = open(&args.b)?;

    // Fail fast (before creating the output) when two regular files differ in size.
    if args.mode == Mode::Strict
        && let (Ok(ma), Ok(mb)) = (a.metadata(), b.metadata())
        && ma.is_file()
        && mb.is_file()
        && ma.len() != mb.len()
    {
        return Err(format!(
            "input files differ in length ({} vs {} bytes); use --truncate or --repeat",
            ma.len(),
            mb.len()
        ));
    }

    // Repeat mode keeps the whole key in memory.
    let key = if args.mode == Mode::Repeat {
        let mut k = Vec::new();
        b.read_to_end(&mut k)
            .map_err(|e| format!("cannot read {}: {e}", args.b.display()))?;
        k
    } else {
        Vec::new()
    };

    let mut out = File::create(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    let result = match args.mode {
        Mode::Strict | Mode::Truncate => xor_streams(&mut a, &mut b, &mut out, args.mode),
        Mode::Repeat => xor_repeating(&mut a, &key, &mut out),
    };

    match result {
        Ok(n) => Ok(n),
        Err(e) => {
            // Don't leave a partial result behind. Close the handle first
            // (required on Windows) and only delete regular files.
            drop(out);
            if fs::metadata(&args.out).is_ok_and(|m| m.is_file()) {
                let _ = fs::remove_file(&args.out);
            }
            Err(e.to_string())
        }
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(msg) => {
            eprintln!("xorfiles: {msg}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match run(&args) {
        Ok(n) => {
            println!("wrote {n} bytes to {}", args.out.display());
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("xorfiles: {msg}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn xors_equal_length_inputs() {
        let mut out = Vec::new();
        let n = xor_streams(
            &mut Cursor::new(vec![0xFF, 0x0F, 0xAA]),
            &mut Cursor::new(vec![0x0F, 0x0F, 0x55]),
            &mut out,
            Mode::Strict,
        )
        .unwrap();
        assert_eq!(n, 3);
        assert_eq!(out, vec![0xF0, 0x00, 0xFF]);
    }

    #[test]
    fn strict_rejects_length_mismatch() {
        let mut out = Vec::new();
        let res = xor_streams(
            &mut Cursor::new(vec![1, 2, 3]),
            &mut Cursor::new(vec![1, 2]),
            &mut out,
            Mode::Strict,
        );
        assert!(res.is_err());
    }

    #[test]
    fn truncate_stops_at_shorter_input() {
        let mut out = Vec::new();
        let n = xor_streams(
            &mut Cursor::new(vec![1, 2, 3]),
            &mut Cursor::new(vec![1, 1]),
            &mut out,
            Mode::Truncate,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(out, vec![0, 3]);
    }

    #[test]
    fn repeat_cycles_the_key() {
        let mut out = Vec::new();
        let n = xor_repeating(
            &mut Cursor::new(vec![1, 2, 3, 4, 5]),
            &[0xFF, 0x00],
            &mut out,
        )
        .unwrap();
        assert_eq!(n, 5);
        assert_eq!(out, vec![0xFE, 0x02, 0xFC, 0x04, 0xFA]);
    }

    #[test]
    fn repeat_rejects_empty_key() {
        let mut out = Vec::new();
        let res = xor_repeating(&mut Cursor::new(vec![1, 2, 3]), &[], &mut out);
        assert!(res.is_err());
    }

    #[test]
    fn works_across_buffer_boundaries() {
        let data = vec![0xABu8; BUF_SIZE * 2 + 7];
        let mut out = Vec::new();
        let n = xor_streams(
            &mut Cursor::new(data.clone()),
            &mut Cursor::new(data.clone()),
            &mut out,
            Mode::Strict,
        )
        .unwrap();
        assert_eq!(n as usize, data.len());
        assert!(out.iter().all(|&b| b == 0));
    }
}
