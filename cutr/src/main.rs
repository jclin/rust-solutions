use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::NonZeroUsize;
use std::ops::Range;

use anyhow::{Result, anyhow, bail};
use clap::Parser;
use csv::{ReaderBuilder, StringRecord};

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
    #[arg(value_name = "FILES", default_value = "-")]
    files: Vec<String>,

    /// Field delimiter
    #[arg(short('d'), long("delimiter"), default_value = "\t")]
    delimiter: String,

    #[command(flatten)]
    extract: ArgsExtract,
}

#[derive(Debug, clap::Args)]
#[group(required(true), multiple(false))]
struct ArgsExtract {
    /// Selected fields
    #[arg(short('f'), long("fields"))]
    fields: Option<String>,

    /// Selected bytes
    #[arg(short('b'), long("bytes"))]
    bytes: Option<String>,

    /// Selected chars
    #[arg(short('c'), long("chars"))]
    chars: Option<String>,
}

type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

fn main() {
    if let Err(err) = run(Args::parse()) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let delim_bytes = args.delimiter.as_bytes();
    if delim_bytes.len() != 1 {
        bail!(r#"--delim "{}" must be a single byte"#, args.delimiter);
    }

    let delimiter = delim_bytes[0];

    let extract = if let Some(fields) = args.extract.fields.map(parse_pos).transpose()? {
        Extract::Fields(fields)
    } else if let Some(bytes) = args.extract.bytes.map(parse_pos).transpose()? {
        Extract::Bytes(bytes)
    } else if let Some(chars) = args.extract.chars.map(parse_pos).transpose()? {
        Extract::Chars(chars)
    } else {
        unreachable!("Must have --fields, --bytes, or --chars");
    };

    for filename in &args.files {
        match open(filename) {
            Err(err) => eprintln!("{filename}: {err}"),
            Ok(file) => match &extract {
                Extract::Chars(ranges) => {
                    for line in file.lines() {
                        println!("{}", extract_chars(&line?, ranges))
                    }
                }
                Extract::Bytes(ranges) => {
                    for line in file.lines() {
                        println!("{}", extract_bytes(&line?, ranges))
                    }
                }
                Extract::Fields(ranges) => {
                    let mut reader = ReaderBuilder::new()
                        .delimiter(delimiter)
                        .has_headers(false)
                        .from_reader(file);

                    for record in reader.records() {
                        let r = &record?;
                        let fields = extract_fields(r, ranges);
                        println!("{}", fields.join(&String::from_utf8(vec![delimiter])?))
                    }
                }
            },
        }
    }

    Ok(())
}

fn parse_pos(range: String) -> Result<PositionList> {
    range
        .split(',')
        .map(|value| {
            parse_index(value).map(|n| n..n + 1).or_else(|e| {
                split_range(value).ok_or(e).and_then(|split| {
                    let idx1 = parse_index(split.0)?;
                    let idx2 = parse_index(split.1)?;
                    if idx1 >= idx2 {
                        bail!(
                            "First number in range ({}) \
                            must be lower than second number ({})",
                            idx1 + 1,
                            idx2 + 1
                        );
                    }
                    Ok(idx1..idx2 + 1)
                })
            })
        })
        .collect()
}

fn parse_index(value: &str) -> Result<usize> {
    let value_error = || anyhow!("illegal list value: \"{value}\"");

    value
        .chars()
        .any(|char| !char.is_ascii_digit())
        .then(|| Err(value_error()))
        .unwrap_or_else(|| {
            value
                .parse::<NonZeroUsize>()
                .map(|number| usize::from(number) - 1)
                .map_err(|_| value_error())
        })
}

fn split_range(value: &str) -> Option<(&str, &str)> {
    value.split_once('-').filter(|(a, b)| {
        a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|b| b.is_ascii_digit())
    })
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let chars: Vec<_> = line.chars().collect();
    char_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| chars.get(i)))
        .collect()
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let bytes = line.as_bytes();
    let slice = byte_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| bytes.get(i)).copied())
        .collect::<Vec<_>>();

    String::from_utf8_lossy(&slice).into_owned()
}

fn extract_fields<'a>(record: &'a StringRecord, field_pos: &[Range<usize>]) -> Vec<&'a str> {
    field_pos
        .iter()
        .cloned()
        .flat_map(|field_pos| field_pos.filter_map(|i| record.get(i)))
        .collect()
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

#[cfg(test)]
mod unit_tests {
    use csv::StringRecord;

    use super::{extract_bytes, extract_chars, extract_fields, parse_pos};

    #[test]
    fn test_parse_pos() {
        // The empty string is an error
        assert!(parse_pos("".to_string()).is_err());

        // Zero is an error
        let res = parse_pos("0".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "0""#);

        let res = parse_pos("0-1".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "0""#);

        // A leading "+" is an error
        let res = parse_pos("+1".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "+1""#,);

        let res = parse_pos("+1-2".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "+1-2""#,
        );

        let res = parse_pos("1-+2".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "1-+2""#,
        );

        // Any non-number is an error
        let res = parse_pos("a".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "a""#);

        let res = parse_pos("1,a".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "a""#);

        let res = parse_pos("1-a".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "1-a""#,);

        let res = parse_pos("a-1".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"illegal list value: "a-1""#,);

        // Wonky ranges
        let res = parse_pos("-".to_string());
        assert!(res.is_err());

        let res = parse_pos(",".to_string());
        assert!(res.is_err());

        let res = parse_pos("1,".to_string());
        assert!(res.is_err());

        let res = parse_pos("1-".to_string());
        assert!(res.is_err());

        let res = parse_pos("1-1-1".to_string());
        assert!(res.is_err());

        let res = parse_pos("1-1-a".to_string());
        assert!(res.is_err());

        // First number must be less than second
        let res = parse_pos("1-1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // All the following are acceptable
        let res = parse_pos("1".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("ábc", &[0..1]), "á".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 2..3]), "ác".to_string());
        assert_eq!(extract_chars("ábc", &[0..3]), "ábc".to_string());
        assert_eq!(extract_chars("ábc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 1..2, 4..5]), "áb".to_string());
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2, 5..6]), "á".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}
