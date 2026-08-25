use std::io::{BufRead, BufReader};

use anyhow::Result;
use clap::Parser;

const DASH: &str = "-";

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
    #[arg(value_name("FILE"), default_value = "-")]
    files: Vec<String>,

    /// Show line count
    #[arg(short('l'), long("lines"))]
    lines: bool,

    /// Show word count
    #[arg(short('w'), long("words"))]
    words: bool,

    /// Show byte count
    #[arg(short('c'), long("bytes"), conflicts_with("chars"))]
    bytes: bool,

    /// Show character count
    #[arg(short('m'), long("chars"))]
    chars: bool,
}

#[derive(Debug, PartialEq)]
struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

impl FileInfo {
    fn flatten(self, args: &Args) -> Vec<usize> {
        let mut counts: Vec<usize> = Vec::new();
        if args.lines {
            counts.push(self.num_lines);
        }
        if args.words {
            counts.push(self.num_words);
        }
        if args.chars {
            counts.push(self.num_chars);
        }
        if args.bytes {
            counts.push(self.num_bytes);
        }

        counts
    }
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        DASH => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(std::fs::File::open(filename)?))),
    }
}

fn count(mut file: impl BufRead) -> Result<FileInfo> {
    let mut tot_bytes = 0;
    let mut tot_lines = 0;
    let mut tot_words = 0;
    let mut tot_chars = 0;

    let mut line_buf = String::new();
    for _ in 0.. {
        let bytes_read = file.read_line(&mut line_buf)?;
        if bytes_read == 0 {
            break;
        }

        tot_bytes += bytes_read;
        tot_lines += 1;

        let words = line_buf.split_whitespace();
        tot_words += words.count();

        tot_chars += line_buf.chars().count();

        line_buf.clear();
    }

    Ok(FileInfo {
        num_lines: tot_lines,
        num_words: tot_words,
        num_bytes: tot_bytes,
        num_chars: tot_chars,
    })
}

fn print_counts(counts: &[usize], text: &str) {
    let result = counts
        .iter()
        .map(|c| format!("{:>8}", c))
        .collect::<Vec<_>>()
        .join("");

    println!("{result}{}{text}", if text.is_empty() { "" } else { " " });
}

fn run(mut args: Args) -> Result<()> {
    if [args.lines, args.words, args.bytes, args.chars]
        .iter()
        .all(|a| *a == false)
    {
        args.lines = true;
        args.words = true;
        args.bytes = true;
    }

    let mut file_counts = FileInfo {
        num_lines: 0,
        num_words: 0,
        num_chars: 0,
        num_bytes: 0,
    };

    for filename in &args.files {
        let fname = if filename == DASH { "" } else { filename };
        match open(filename) {
            Err(err) => eprintln!("{fname}: {err}"),
            Ok(file) => match count(file) {
                Err(err) => println!("{fname}: {err}"),
                Ok(info) => {
                    file_counts.num_lines += info.num_lines;
                    file_counts.num_words += info.num_words;
                    file_counts.num_chars += info.num_chars;
                    file_counts.num_bytes += info.num_bytes;

                    print_counts(&info.flatten(&args), fname);
                }
            },
        }
    }

    if args.files.len() > 1 {
        print_counts(&file_counts.flatten(&args), "total");
    }

    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
