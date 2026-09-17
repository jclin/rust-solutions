use std::fs::{self, File};
use std::io::SeekFrom;
use std::{
    io::{BufRead, BufReader, Read, Seek},
    ops::Neg,
};

use anyhow::{Result, anyhow};
use clap::Parser;
use linecount::count_lines;

use crate::TakeValue::{PlusZero, TakeNum};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = env!("CARGO_PKG_DESCRIPTION"),
    help_template = "\
{before-help}{about-with-newline}
{name} {version}
{author-with-newline}
{usage-heading} {usage}

{all-args}{after-help}"
)]
struct Args {
    /// Input file(s)
    #[arg(value_name = "FILE", required(true))]
    files: Vec<String>,

    /// Number of lines
    #[arg(
        short('n'),
        long("lines"),
        default_value = "10",
        conflicts_with = "bytes"
    )]
    lines: String,

    /// Number of bytes
    #[arg(short('c'), long("bytes"))]
    bytes: Option<String>,

    /// Suppress headers
    #[arg(short('q'), long("quiet"))]
    quiet: bool,
}

#[derive(Debug, PartialEq)]
enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let lines = parse_num(args.lines).map_err(|e| anyhow!("illegal line count -- {e}"))?;

    let bytes = args
        .bytes
        .map(parse_num)
        .transpose()
        .map_err(|e| anyhow!("illegal byte count -- {e}"))?;

    for (index, filename) in args.files.iter().enumerate() {
        match open(filename) {
            Err(e) => eprintln!("{filename}: {e}"),
            Ok(file) => {
                if !args.quiet && args.files.len() > 1 {
                    if index > 0 && index < args.files.len() {
                        println!("");
                    }
                    println!("==> {filename} <==");
                }

                let (num_lines, num_bytes) = count_lines_bytes(filename)?;
                if let Some(take_bytes) = &bytes {
                    print_bytes(file, &take_bytes, num_bytes)?
                } else {
                    print_lines(file, &lines, num_lines)?
                }
            }
        }
    }

    Ok(())
}

fn parse_num(val: String) -> Result<TakeValue> {
    let mut num = val.parse::<i64>().map_err(|_| anyhow!("{val}"))?;
    num = if val.starts_with("+") {
        num.abs()
    } else if val.starts_with("-") {
        num
    } else {
        num.neg()
    };

    if val.starts_with("+") && num == 0 {
        Ok(PlusZero)
    } else {
        Ok(TakeNum(num))
    }
}

fn count_lines_bytes(filename: &str) -> Result<(i64, i64)> {
    let num_lines = { count_lines(open(filename)?)? };

    let metadata = fs::metadata(filename)?;

    Ok((num_lines.try_into()?, metadata.len().try_into()?))
}

fn open(filename: &str) -> Result<BufReader<File>> {
    Ok(BufReader::new(File::open(filename)?))
}

fn print_lines(mut file: impl BufRead, num_lines: &TakeValue, total_lines: i64) -> Result<()> {
    let start_index = get_start_index(num_lines, total_lines);
    if start_index.is_none() {
        return Ok(());
    }

    let start_index = start_index.unwrap();
    let mut line = String::new();
    for index in 0..(total_lines as u64) {
        let bytes_read = file.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        if index < start_index {
            line.clear();
            continue;
        }

        print!("{line}");

        line.clear();
    }

    Ok(())
}

fn print_bytes<T>(mut file: T, num_bytes: &TakeValue, total_bytes: i64) -> Result<()>
where
    T: Read + Seek,
{
    let start_byte = get_start_index(num_bytes, total_bytes);
    if start_byte.is_none() {
        return Ok(());
    }

    let start_byte = start_byte.unwrap();
    file.seek(SeekFrom::Start(start_byte))?;

    let mut buffer: Vec<u8> = Vec::new();
    let bytes_read = file.read_to_end(&mut buffer)?;
    if bytes_read > 0 {
        print!("{}", String::from_utf8_lossy(&buffer));
    }

    Ok(())
}

fn get_start_index(take_val: &TakeValue, total: i64) -> Option<u64> {
    match take_val {
        PlusZero => {
            if total <= 0 {
                None
            } else {
                Some(0)
            }
        }
        TakeNum(number) => match number {
            1..=i64::MAX => {
                if *number > total {
                    None
                } else {
                    Some((number - 1) as u64)
                }
            }
            0 => None,
            i64::MIN..=-1 => {
                if number.abs() <= total {
                    Some((total + *number) as u64)
                } else if number.abs() > total {
                    Some(0)
                } else {
                    None
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{TakeValue::*, count_lines_bytes, get_start_index, parse_num};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        let (lines, bytes) = res.unwrap();
        assert_eq!(lines, 1);
        assert_eq!(bytes, 24);

        let res = count_lines_bytes("tests/inputs/twelve.txt");
        assert!(res.is_ok());
        let (lines, bytes) = res.unwrap();
        assert_eq!(lines, 12);
        assert_eq!(bytes, 63);
    }

    #[test]
    fn test_get_start_index() {
        // +0 from an empty file (0 lines/bytes) returns None
        assert_eq!(get_start_index(&PlusZero, 0), None);

        // +0 from a nonempty file returns an index that
        // is one less than the number of lines/bytes
        assert_eq!(get_start_index(&PlusZero, 1), Some(0));

        // Taking 0 lines/bytes returns None
        assert_eq!(get_start_index(&TakeNum(0), 1), None);

        // Taking any lines/bytes from an empty file returns None
        assert_eq!(get_start_index(&TakeNum(1), 0), None);

        // Taking more lines/bytes than is available returns None
        assert_eq!(get_start_index(&TakeNum(2), 1), None);

        // When starting line/byte is less than total lines/bytes,
        // return one less than starting number
        assert_eq!(get_start_index(&TakeNum(1), 10), Some(0));
        assert_eq!(get_start_index(&TakeNum(2), 10), Some(1));
        assert_eq!(get_start_index(&TakeNum(3), 10), Some(2));

        // When starting line/byte is negative and less than total,
        // return total - start
        assert_eq!(get_start_index(&TakeNum(-1), 10), Some(9));
        assert_eq!(get_start_index(&TakeNum(-2), 10), Some(8));
        assert_eq!(get_start_index(&TakeNum(-3), 10), Some(7));

        // When the starting line/byte is negative and more than the total,
        // return 0 to print the whole file
        assert_eq!(get_start_index(&TakeNum(-20), 10), Some(0));
    }

    #[test]
    fn test_parse_num() {
        // All integers should be interpreted as negative numbers
        let res = parse_num("3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // A leading "+" should result in a positive number
        let res = parse_num("+3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));

        // An explicit "-" value should result in a negative number
        let res = parse_num("-3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // Zero is zero
        let res = parse_num("0".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));

        // Plus zero is special
        let res = parse_num("+0".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);

        // Test boundaries
        let res = parse_num(i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num((i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));

        let res = parse_num(i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));

        // A floating-point value is invalid
        let res = parse_num("3.14".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");

        // Any non-integer string is invalid
        let res = parse_num("foo".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "foo");
    }
}
