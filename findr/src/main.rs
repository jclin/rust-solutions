use anyhow::Result;
use clap::{Parser, ValueEnum, builder::PossibleValue};
use regex::Regex;
use std::{fs::FileType, os::unix::fs::PermissionsExt};
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
    /// Search paths
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<String>,

    /// Names
    #[arg(
        short('n'),
        long("name"),
        value_name = "NAME",
        value_parser(Regex::new),
        num_args(0..)
    )]
    names: Vec<Regex>,

    /// Entry types
    #[arg(
        short('t'),
        long("type"),
        value_name = "TYPE",
        value_parser(clap::value_parser!(EntryType)),
        num_args(0..)
    )]
    entry_types: Vec<EntryType>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum EntryType {
    Dir,
    File,
    Link,
}

impl ValueEnum for EntryType {
    fn value_variants<'a>() -> &'a [Self] {
        return &[EntryType::Dir, EntryType::File, EntryType::Link];
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            EntryType::Dir => PossibleValue::new("d"),
            EntryType::File => PossibleValue::new("f"),
            EntryType::Link => PossibleValue::new("l"),
        })
    }
}

impl EntryType {
    fn from(entry: FileType) -> Option<Self> {
        if entry.is_dir() {
            return Some(EntryType::Dir);
        } else if entry.is_file() {
            return Some(EntryType::File);
        } else if entry.is_symlink() {
            return Some(EntryType::Link);
        }
        None
    }
}

fn main() {
    if let Err(err) = run(Args::parse()) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let has_entry_type = |et: &Option<EntryType>| -> bool {
        if args.entry_types.len() == 0 {
            return true;
        }

        et.is_some() && args.entry_types.contains(et.as_ref().unwrap())
    };

    let is_match = |text: &str| -> bool {
        args.names.len() == 0 || args.names.iter().any(|re| re.is_match(text))
    };

    for path in &args.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Err(err) => eprintln!("{err}"),
                Ok(entry) => {
                    // if entry.metadata()?.permissions().mode() == 0 {
                    //     continue;
                    // }

                    let path = entry.path();
                    let Some(file_name) = path.file_name() else {
                        continue;
                    };
                    // let leaf = format!("{}", file_name.display());
                    let leaf = file_name.to_string_lossy();

                    if has_entry_type(&EntryType::from(entry.file_type())) && is_match(&leaf) {
                        println!("{}", path.display())
                    }
                }
            }
        }
    }
    Ok(())
}
