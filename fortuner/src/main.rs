use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::{Ok, Result, anyhow};
use clap::Parser;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng, seq::SliceRandom};
use regex::RegexBuilder;
use walkdir::WalkDir;

const SEP: &str = "\n%";

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
    /// Input files or directories
    #[arg(value_name = "FILE", required = true)]
    sources: Vec<String>,

    /// Pattern
    #[arg(short('m'), long("pattern"))]
    pattern: Option<String>,

    /// Case-insensitive pattern matching
    #[arg(short('i'), long("insensitive"))]
    insensitive: bool,

    /// Random seed
    #[arg(short('s'), long("seed"))]
    seed: Option<u64>,
}

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let pattern = args
        .pattern
        .map(|val| {
            RegexBuilder::new(val.as_str())
                .case_insensitive(args.insensitive)
                .build()
                .map_err(|_| anyhow!(r#"Invalid --pattern "{val}""#))
        })
        .transpose()?;

    let files = find_files(&args.sources)?;
    let fortunes = read_fortunes(&files)?;

    match pattern {
        Some(pattern) => {
            let mut curr_source = String::new();
            for fortune in &fortunes {
                if !pattern.is_match(&fortune.text) {
                    continue;
                }

                if curr_source != fortune.source {
                    curr_source = fortune.source.clone();
                    eprintln!("({curr_source}){SEP}")
                }
                println!("{}{SEP}", fortune.text);
            }
        }
        _ => {
            if let Some(fortune) = pick_fortune(&fortunes, args.seed) {
                println!("{fortune}");
            } else {
                println!("No fortunes found");
            }
        }
    }

    Ok(())
}

fn find_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut path_bufs: Vec<PathBuf> = paths
        .iter()
        .flat_map(|p| WalkDir::new(p))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .map(|entry| entry.path().to_path_buf())
        .filter(|path_buf| path_buf.is_file())
        .collect();

    path_bufs.sort();
    path_bufs.dedup();

    Ok(path_bufs)
}

fn read_fortunes(paths: &[PathBuf]) -> Result<Vec<Fortune>> {
    let mut fortunes: Vec<Fortune> = Vec::new();
    for path in paths {
        let mut file =
            open(path).map_err(|e| anyhow!("{}: {e}", path.file_name().unwrap().display()))?;
        let mut fortune = String::new();
        let mut line_buf = String::new();

        loop {
            let bytes_read = file.read_line(&mut line_buf)?;
            if bytes_read == 0 {
                break;
            }

            if line_buf.trim() == "%" {
                if !fortune.trim().is_empty() {
                    fortunes.push(Fortune {
                        source: path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        text: fortune.trim().to_string(),
                    });
                    fortune.clear();
                }
            } else {
                fortune = fortune + &line_buf;
            }

            line_buf.clear();
        }
    }

    Ok(fortunes)
}

fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    let mut rng: Box<dyn Rng> = if seed.is_some() {
        Box::new(rand::rngs::StdRng::seed_from_u64(seed.unwrap()))
    } else {
        Box::new(rand::rng())
    };

    fortunes
        .choose(&mut rng)
        .map(|fortune| fortune.text.clone())
}

fn open(filename: &PathBuf) -> Result<Box<dyn BufRead>> {
    Ok(Box::new(BufReader::new(File::open(filename)?)))
}

#[cfg(test)]
mod tests {
    use super::{Fortune, find_files, pick_fortune, read_fortunes};
    use std::path::PathBuf;

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.get(0).unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );

        // Fails to find a bad file
        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());

        // Finds all the input files, excludes ".dat"
        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());

        // Check number and order of files
        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.get(0).unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));

        // Test for multiple sources, path must be unique and sorted
        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string())
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string())
        }
    }

    #[test]
    fn test_read_fortunes() {
        // Parses all the fortunes without a filter
        let res = read_fortunes(&[PathBuf::from("./tests/inputs/jokes")]);
        assert!(res.is_ok());

        if let Ok(fortunes) = res {
            // Correct number and sorting
            assert_eq!(fortunes.len(), 6);
            assert_eq!(
                fortunes.first().unwrap().text,
                "Q. What do you call a head of lettuce in a shirt and tie?\n\
                A. Collared greens."
            );
            assert_eq!(
                fortunes.last().unwrap().text,
                "Q: What do you call a deer wearing an eye patch?\n\
                A: A bad idea (bad-eye deer)."
            );
        }

        // Filters for matching text
        let res = read_fortunes(&[
            PathBuf::from("./tests/inputs/jokes"),
            PathBuf::from("./tests/inputs/quotes"),
        ]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 11);
    }

    #[test]
    fn test_pick_fortune() {
        // Create a slice of fortunes
        let fortunes = &[
            Fortune {
                source: "fortunes".to_string(),
                text: "You cannot achieve the impossible without \
                      attempting the absurd."
                    .to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Assumption is the mother of all screw-ups.".to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Neckties strangle clear thinking.".to_string(),
            },
        ];

        // Pick a fortune with a seed
        assert_eq!(
            pick_fortune(fortunes, Some(1)).unwrap(),
            "Neckties strangle clear thinking.".to_string()
        );
    }
}
