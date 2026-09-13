use std::io::{BufRead, BufReader};

use anyhow::{Result, anyhow};
use clap::Parser;
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

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
    /// Search pattern
    #[arg(value_name = "PATTERN")]
    pattern: String,

    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    files: Vec<String>,

    /// Case insensitive
    #[arg(short('i'), long("insensitive"))]
    insensitive: bool,

    /// Recursive search
    #[arg(short('r'), long("recursive"))]
    recursive: bool,

    /// Count occurrences
    #[arg(short('c'), long("count"))]
    count: bool,

    /// Invert match
    #[arg(short('v'), long("invert-match"))]
    invert: bool,
}

const DASH: &str = "-";

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let pattern = RegexBuilder::new(&args.pattern)
        .case_insensitive(args.insensitive)
        .build()
        .map_err(|_| anyhow!(r#"Invalid pattern "{}""#, args.pattern))?;

    let multiple_files = |entries: &[Result<String>]| {
        entries.len() > 1
    };

    let entries = find_files(&args.files, args.recursive);
    for entry in &entries {
        match entry {
            Err(e) => eprintln!("{e}"),
            Ok(filename) => match open(&filename) {
                Err(e) => eprintln!("{filename}: {e}"),
                Ok(file) => {
                    let matches = find_lines(file, &pattern, args.invert)?;

                    let prefix = if multiple_files(&entries) {
                        format!("{filename}:")
                    } else {
                        "".to_string()
                    };

                    if args.count {
                        print!("{prefix}{}\n", matches.len())
                    } else {
                        matches.iter()
                            .for_each(|m| print!("{prefix}{m}"))
                    };
                }
            },
        }
    }

    Ok(())
}

fn find_files(paths: &[String], recursive: bool) -> Vec<Result<String>> {
    let mut results: Vec<Result<String>> = Vec::new();

    for path in paths {
        if path == DASH {
            results.push(Ok("-".to_string()));
            continue;
        }

        let walker = WalkDir::new(path);

        for entry in walker {
            match entry {
                Err(err) => results.push(Err(anyhow!("{err}"))),
                Ok(dir_entry) => {
                    if dir_entry.path().is_dir() {
                        if !recursive {
                            results.push(Err(anyhow!("{path} is a directory")));
                            break;
                        }
                    } else if dir_entry.path().is_file() {
                        results.push(Ok(dir_entry.path().to_string_lossy().to_string()))
                    }
                }
            }
        }
    }

    results
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(std::fs::File::open(filename)?))),
    }
}

fn find_lines<T: BufRead>(mut file: T, pattern: &Regex, invert: bool) -> Result<Vec<String>> {
    let mut matches: Vec<String> = Vec::new();
    let mut line = String::new();

    loop {
        let bytes_read = file.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        let is_matched = pattern.is_match(&line);
        if (is_matched && !invert) || (!is_matched && invert) {
            matches.push(line.clone());
        }

        line.clear();
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::{find_files, find_lines};
    use pretty_assertions::assert_eq;
    use rand::{RngExt, distr::Alphanumeric};
    use regex::{Regex, RegexBuilder};
    use std::io::Cursor;

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";

        // The pattern _or_ should match the one line, "Lorem"
        let re1 = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&text), &re1, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);

        // When inverted, the function should match the other two lines
        let matches = find_lines(Cursor::new(&text), &re1, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // This regex will be case-insensitive
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();

        // The two lines "Lorem" and "DOLOR" should match
        let matches = find_lines(Cursor::new(&text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // When inverted, the one remaining line should match
        let matches = find_lines(Cursor::new(&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_ref().unwrap(), "./tests/inputs/fox.txt");

        // The function should reject a directory without the recursive option
        let files = find_files(&["./tests/inputs".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert_eq!(e.to_string(), "./tests/inputs is a directory");
        }

        // Verify the function recurses to find four files in the directory
        let res = find_files(&["./tests/inputs".to_string()], true);
        let mut files: Vec<String> = res
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 4);
        assert_eq!(
            files,
            vec![
                "./tests/inputs/bustle.txt",
                "./tests/inputs/empty.txt",
                "./tests/inputs/fox.txt",
                "./tests/inputs/nobody.txt",
            ]
        );

        // Generate a random string to represent a nonexistent file
        let bad: String = rand::rng()
            .sample_iter(Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        // Verify that the function returns the bad file as an error
        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }
}
