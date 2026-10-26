use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::process;

const BUFFER_SIZE: usize = 64 * 1024;

fn fill_buffer<R: Read>(reader: &mut R, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;

    while filled < buffer.len() {
        let n = reader.read(&mut buffer[filled..])?;

        if n == 0 {
            break;
        }

        filled += n;
    }

    Ok(filled)
}

fn xor_files(input1: &str, input2: &str, output: &str) -> io::Result<()> {
    let file1 = File::open(input1)?;
    let file2 = File::open(input2)?;

    // Avoid accidentally truncating an input file when paths resolve identically.
    let output_path = std::path::Path::new(output);
    if output_path.exists() {
        let out = output_path.canonicalize()?;
        let in1 = std::path::Path::new(input1).canonicalize()?;
        let in2 = std::path::Path::new(input2).canonicalize()?;

        if out == in1 || out == in2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output file must be different from both input files",
            ));
        }
    }

    let mut reader1 = BufReader::new(file1);
    let mut reader2 = BufReader::new(file2);
    let output_file = File::create(output)?;
    let mut writer = BufWriter::new(output_file);

    let mut buffer1 = [0u8; BUFFER_SIZE];
    let mut buffer2 = [0u8; BUFFER_SIZE];
    let mut result = [0u8; BUFFER_SIZE];

    loop {
        let n1 = fill_buffer(&mut reader1, &mut buffer1)?;
        let n2 = fill_buffer(&mut reader2, &mut buffer2)?;
        let n = n1.min(n2);

        if n == 0 {
            break;
        }

        for i in 0..n {
            result[i] = buffer1[i] ^ buffer2[i];
        }

        writer.write_all(&result[..n])?;

        // Stop when either input reaches EOF; unmatched trailing bytes are ignored.
        if n1 < BUFFER_SIZE || n2 < BUFFER_SIZE {
            break;
        }
    }

    writer.flush()?;
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("Usage: xor-files <input1> <input2> <output>");
        println!("XORs corresponding bytes up to the shorter input's length.");
        return;
    }

    if args.len() != 3 {
        eprintln!("Usage: xor-files <input1> <input2> <output>");
        eprintln!("Run 'xor-files --help' for more information.");
        process::exit(2);
    }

    if let Err(err) = xor_files(&args[0], &args[1], &args[2]) {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
