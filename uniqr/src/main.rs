use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::fs::File;

use anyhow::{Ok, anyhow};
use anyhow::Result;
use clap::{Parser};

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
    /// Input file
    #[arg(value_name = "IN_FILE", default_value = "-")]
    in_file: String,

    /// Output file
    #[arg(value_name = "OUT_FILE")]
    out_file: Option<String>,

    /// prefix lines by the number of occurrences
    #[arg(short('c'), long("count"))]
    count: bool,
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?)))
    }
}

fn create(filename: &Option<String>) -> Result<Box<dyn Write>> {
    match filename {
        Some(filename) => Ok(Box::new(BufWriter::new(File::create(filename)?))),
        _ => Ok(Box::new(BufWriter::new(io::stdout()))),
    }
}

fn run(args: Args) -> Result<()> {
    let mut file = open(&args.in_file)
        .map_err(|err| anyhow!("{}: {err}", args.in_file))?;

    let mut out_file = create(&args.out_file)
        .map_err(|err| anyhow!("{}: {err}", if args.out_file.is_none() { "stdout".to_string() } else { args.out_file.unwrap() }))?;

    let mut print = |count: usize, line: &str| -> Result<()> {
        if args.count && count > 0 {
            write!(&mut out_file, "{count:>4} {line}")?
        } else {
            write!(&mut out_file, "{line}")?
        }

        Ok(())
    };

    let mut count = 0_usize;
    let mut prev = String::new();
    let mut line = String::new();
    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        if prev.trim_end() != line.trim_end() {
            print(count, &prev)?;
            prev = line.clone();
            count = 0;
        }

        count += 1;
        line.clear();
    }

    print(count, &prev)?;

    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
