use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Local};
use clap::Parser;
use tabular::{Row, Table};
use users::{get_group_by_gid, get_user_by_uid};

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
    /// Files and/or directories
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<String>,

    /// Long listing
    #[arg(short('l'), long("long"))]
    long: bool,

    /// Show all files
    #[arg(short('a'), long("all"))]
    show_hidden: bool,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let paths = find_files(&args.paths, args.show_hidden)?;
    if args.long {
        println!("{}", format_output(&paths)?);
    } else {
        paths.iter().for_each(|path| println!("{}", path.display()));
    }

    Ok(())
}

fn find_files(paths: &[String], show_hidden: bool) -> Result<Vec<PathBuf>> {
    let mut results: Vec<PathBuf> = Vec::new();
    let mut try_push = |path: PathBuf, metadata: Metadata, show_hidden: bool| {
        if metadata.is_file() {
            let hidden = path
                .file_name()
                .map_or(false, |fname| fname.to_string_lossy().starts_with("."));
            if !hidden || show_hidden {
                results.push(PathBuf::from(path));
            }
            return;
        }
        if metadata.is_dir() {
            results.push(path);
        }
    };

    for path in paths {
        let result = fs::metadata(path);
        let Ok(metadata) = result else {
            eprintln!("{path}: {}", result.unwrap_err());
            continue;
        };

        if metadata.is_file() {
            try_push(PathBuf::from(path), metadata, true);
            continue;
        }

        if metadata.is_dir() {
            let dir_result = fs::read_dir(path);
            let Ok(dir) = dir_result else {
                eprintln!("{}", dir_result.unwrap_err());
                continue;
            };

            for entry in dir {
                let Ok(path) = entry else {
                    eprintln!("{}", entry.unwrap_err());
                    continue;
                };

                let metadata_result = path.metadata();
                let Ok(metadata) = metadata_result else {
                    eprintln!("{}", metadata_result.unwrap_err());
                    continue;
                };

                try_push(path.path(), metadata, show_hidden);
            }
        }
    }

    Ok(results)
}

fn format_output(paths: &[PathBuf]) -> Result<String> {
    //         1   2     3     4     5     6     7     8
    let fmt = "{:<}{:<}  {:>}  {:<}  {:<}  {:>}  {:<}  {:<}";
    let mut table = Table::new(fmt);

    for path in paths {
        let metadata = path.metadata()?;
        let entry_type = if metadata.is_dir() { "d" } else { "-" };
        let mode = format_mode(metadata.mode());
        let num_links = format!("{}", metadata.nlink());
        let username = get_user_by_uid(metadata.uid())
            .map_or("".to_string(), |m| m.name().to_string_lossy().to_string());
        let group = get_group_by_gid(metadata.gid()).map_or("".to_string(), |group| {
            group.name().to_string_lossy().to_string()
        });
        let size = format!("{}", metadata.size());

        let dt: DateTime<Local> = metadata.modified()?.into();
        let modified = format!("{}", dt.format("%b %d %C %H:%M"));

        table.add_row(
            Row::new()
                .with_cell(entry_type) // 1 "d" or "-"
                .with_cell(mode) // 2 permissions
                .with_cell(num_links) // 3 number of links
                .with_cell(username) // 4 user name
                .with_cell(group) // 5 group name
                .with_cell(size) // 6 size
                .with_cell(modified) // 7 modification
                .with_cell(format!("{}", path.display())), // 8 path
        );
    }

    Ok(format!("{table}"))
}

/// Given a file mode in octal format like 0o751,
/// return a string like "rwxr-x--x"
fn format_mode(mode: u32) -> String {
    let mut format = String::new();

    // User
    let user_read = if (mode & 0o400) == 0o400 { "r" } else { "-" };
    let user_write = if (mode & 0o200) == 0o200 { "w" } else { "-" };
    let user_execute = if (mode & 0o100) == 0o100 { "x" } else { "-" };
    format = format + &format!("{user_read}{user_write}{user_execute}");

    // Group
    let group_read = if (mode & 0o040) == 0o040 { "r" } else { "-" };
    let group_write = if (mode & 0o020) == 0o020 { "w" } else { "-" };
    let group_execute = if (mode & 0o010) == 0o010 { "x" } else { "-" };
    format = format + &format!("{group_read}{group_write}{group_execute}");

    // Other
    let other_read = if (mode & 0o004) == 0o004 { "r" } else { "-" };
    let other_write = if (mode & 0o002) == 0o002 { "w" } else { "-" };
    let other_execute = if (mode & 0o001) == 0o001 { "x" } else { "-" };
    format = format + &format!("{other_read}{other_write}{other_execute}");

    format
}

#[cfg(test)]
mod test {
    use super::{find_files, format_mode, format_output};
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    #[test]
    fn test_find_files() {
        // Find all non-hidden entries in a directory
        let res = find_files(&["tests/inputs".to_string()], false);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );

        // Any existing file should be found even if hidden
        let res = find_files(&["tests/inputs/.hidden".to_string()], false);
        assert!(res.is_ok());
        let filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        assert_eq!(filenames, ["tests/inputs/.hidden"]);

        // Test multiple path arguments
        let res = find_files(
            &[
                "tests/inputs/bustle.txt".to_string(),
                "tests/inputs/dir".to_string(),
            ],
            false,
        );
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            ["tests/inputs/bustle.txt", "tests/inputs/dir/spiders.txt"]
        );
    }

    #[test]
    fn test_find_files_hidden() {
        // Find all entries in a directory including hidden
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );
    }

    fn long_match(
        line: &str,
        expected_name: &str,
        expected_perms: &str,
        expected_size: Option<&str>,
    ) {
        let parts: Vec<_> = line.split_whitespace().collect();
        assert!(!parts.is_empty() && parts.len() <= 10);

        let perms = parts.first().unwrap();
        assert_eq!(perms, &expected_perms);

        if let Some(size) = expected_size {
            let file_size = parts.get(4).unwrap();
            assert_eq!(file_size, &size);
        }

        let display_name = parts.last().unwrap();
        assert_eq!(display_name, &expected_name);
    }

    #[test]
    fn test_format_output_one() {
        let bustle_path = "tests/inputs/bustle.txt";
        let bustle = PathBuf::from(bustle_path);

        let res = format_output(&[bustle]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let lines: Vec<&str> = out.split('\n').filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 1);

        let line1 = lines.first().unwrap();
        long_match(line1, bustle_path, "-rw-r--r--", Some("193"));
    }

    #[test]
    fn test_format_output_two() {
        let res = format_output(&[
            PathBuf::from("tests/inputs/dir"),
            PathBuf::from("tests/inputs/empty.txt"),
        ]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let mut lines: Vec<&str> = out.split('\n').filter(|s| !s.is_empty()).collect();
        lines.sort();
        assert_eq!(lines.len(), 2);

        let empty_line = lines.remove(0);
        long_match(
            empty_line,
            "tests/inputs/empty.txt",
            "-rw-r--r--",
            Some("0"),
        );

        let dir_line = lines.remove(0);
        long_match(dir_line, "tests/inputs/dir", "drwxr-xr-x", None);
    }

    #[test]
    fn test_format_mode() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o421), "r---w---x");
    }
}
