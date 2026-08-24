use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};

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
    #[arg(value_name = "FILE", default_value = "-")]
    files: Vec<String>,

    /// print the first NUM lines instead of the first 10
    #[arg(
        short('n'),
        long("lines"),
        default_value("10"),
        conflicts_with("bytes"),
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    lines: u64,

    /// print the first NUM bytes of each file
    #[arg(
        short('c'),
        long("bytes"),
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    bytes: Option<u64>,
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn run(args: Args) -> Result<()> {
    let print_header = args.files.len() > 1;

    for (file_index, filename) in args.files.iter().enumerate() {
        match open(&filename) {
            Err(err) => eprintln!("{filename}: {err}"),
            Ok(mut file) => {
                if print_header {
                    if file_index > 0 && file_index <= args.files.len() - 1 {
                        println!("");
                    }
                    println!("==> {filename} <==");
                }

                match args.bytes {
                    Some(num_bytes) => {
                        let mut handle = file.take(num_bytes);
                        io::copy(&mut handle, &mut io::stdout())?;
                    }
                    None => {
                        let mut line = String::new();
                        for _ in 0..args.lines {
                            let bytes = file.read_line(&mut line)?;
                            if bytes == 0 {
                                break;
                            }

                            print!("{line}");
                            line.clear();
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
