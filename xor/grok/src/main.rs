use std::env;
use std::fs::File;
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::path::Path;
use std::process;

fn usage(program: &str) {
    eprintln!("Usage: {} <file1> <file2> <output>", program);
    eprintln!();
    eprintln!("XOR corresponding bytes of two files and write the result to a third file.");
    eprintln!("If the files differ in length, processing stops at the end of the shorter file.");
}

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "xor_cli".to_string());

    let file1 = match args.next() {
        Some(f) => f,
        None => {
            usage(&program);
            process::exit(1);
        }
    };
    let file2 = match args.next() {
        Some(f) => f,
        None => {
            usage(&program);
            process::exit(1);
        }
    };
    let output = match args.next() {
        Some(f) => f,
        None => {
            usage(&program);
            process::exit(1);
        }
    };

    if args.next().is_some() {
        usage(&program);
        process::exit(1);
    }

    if let Err(e) = run(&file1, &file2, &output) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(file1: &str, file2: &str, output: &str) -> io::Result<()> {
    let mut reader1 = BufReader::new(File::open(Path::new(file1))?);
    let mut reader2 = BufReader::new(File::open(Path::new(file2))?);
    let mut writer = BufWriter::new(File::create(Path::new(output))?);

    let mut buf1 = [0u8; 8192];
    let mut buf2 = [0u8; 8192];

    loop {
        let n1 = reader1.read(&mut buf1)?;
        let n2 = reader2.read(&mut buf2)?;

        let n = n1.min(n2);
        if n == 0 {
            break;
        }

        for i in 0..n {
            buf1[i] ^= buf2[i];
        }

        writer.write_all(&buf1[..n])?;
    }

    writer.flush()?;
    Ok(())
}