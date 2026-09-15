use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader};

use anyhow::{Ok, Result, anyhow, bail};
use clap::{ArgAction, Parser};

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
    /// Input file 1
    #[arg(value_name = "FILE1", required(true))]
    file1: String,

    /// Input file 2
    #[arg(value_name = "FILE2", required(true))]
    file2: String,

    /// Suppress printing of column 1
    #[arg(short('1'), default_value_t = true, action = ArgAction::SetFalse)]
    show_col1: bool,

    /// Suppress printing of column 2
    #[arg(short('2'), default_value_t = true, action = ArgAction::SetFalse)]
    show_col2: bool,

    /// Suppress printing of column 3
    #[arg(short('3'), default_value_t = true, action = ArgAction::SetFalse)]
    show_col3: bool,

    /// Case-insensitive comparison of lines
    #[arg(short('i'))]
    insensitive: bool,

    /// Output delimiter
    #[arg(short('d'), long("output-delimiter"), default_value = "\t")]
    delimiter: String,
}

enum Column<'a> {
    Col1(&'a str),
    Col2(&'a str),
    Col3(&'a str),
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            File::open(filename).map_err(|e| anyhow!("{filename}: {e}"))?,
        ))),
    }
}

fn run(args: Args) -> Result<()> {
    let file1 = &args.file1;
    let file2 = &args.file2;

    if file1 == DASH && file2 == DASH {
        bail!(r#"Both input files cannot be STDIN ("-")"#);
    }

    let case = |line: String| {
        if args.insensitive {
            line.to_lowercase()
        } else {
            line
        }
    };

    let print = |col: Column| {
        let mut output: Vec<&str> = vec![];
        match col {
            Column::Col1(str) => {
                if args.show_col1 {
                    output.push(str)
                }
            }
            Column::Col2(str) => {
                if args.show_col2 {
                    if args.show_col1 {
                        output.push("");
                    }
                    output.push(str);
                }
            }
            Column::Col3(str) => {
                if args.show_col3 {
                    if args.show_col1 {
                        output.push("");
                    }
                    if args.show_col2 {
                        output.push("");
                    }
                    output.push(str);
                }
            }
        }

        if !output.is_empty() {
            println!("{}", output.join(&args.delimiter));
        }
    };

    let mut lines1 = open(file1)?.lines().map_while(Result::ok).map(case);
    let mut lines2 = open(file2)?.lines().map_while(Result::ok).map(case);

    let mut line1 = lines1.next();
    let mut line2 = lines2.next();

    while line1.is_some() || line2.is_some() {
        match (&line1, &line2) {
            (Some(l1), Some(l2)) => match l1.cmp(l2) {
                Ordering::Less => {
                    print(Column::Col1(l1));
                    line1 = lines1.next();
                }
                Ordering::Greater => {
                    print(Column::Col2(l2));
                    line2 = lines2.next();
                }
                Ordering::Equal => {
                    print(Column::Col3(l1));
                    line1 = lines1.next();
                    line2 = lines2.next();
                }
            },
            (Some(l1), None) => {
                print(Column::Col1(l1));
                line1 = lines1.next();
            }
            (None, Some(l2)) => {
                print(Column::Col2(l2));
                line2 = lines2.next();
            }
            _ => (),
        }
    }

    Ok(())
}
