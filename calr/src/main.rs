use ansi_term::Style;
use anyhow::{Result, bail};
use chrono::{Datelike, Days, Local, NaiveDate};
use clap::Parser;
use itertools::{Itertools, izip};

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
    /// Year (1-9999)
    #[arg(
        value_name = "YEAR",
        value_parser = clap::value_parser!(i32).range(1..=9999),
        conflicts_with = "show_current_year"
    )]
    year: Option<i32>,

    /// Month name or number (1-12)
    #[arg(short('m'), conflicts_with = "show_current_year")]
    month: Option<String>,

    /// Show whole current year
    #[arg(short('y'), long("year"))]
    show_current_year: bool,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const CHARS_PER_DAY: usize = 3;
const LINE_LEN: usize = 22;

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let today = Local::now().date_naive();
    let mut month = args.month.map(parse_month).transpose()?;
    let mut year = args.year;

    if args.show_current_year {
        month = None;
        year = Some(today.year());
    } else if month.is_none() && year.is_none() {
        month = Some(today.month());
        year = Some(today.year());
    }

    let year = year.unwrap_or(today.year());

    match month {
        Some(month) => {
            let lines = format_month(year, month, true, today);
            println!("{}", lines.join("\n"));
        },
        None => {
            let lines = format_year(year, today);
            println!("{}", lines.join("\n"));
        },
    }

    Ok(())
}

fn parse_month(month: String) -> Result<u32> {
    match month.parse::<u32>() {
        Err(_e) => {
            let matches: Vec<_> = MONTHS
                .iter()
                .enumerate()
                .filter_map(|(index, name)| {
                    if name.to_lowercase().starts_with(&month.to_lowercase()) {
                        Some(index + 1)
                    } else {
                        None
                    }
                })
                .collect();

            if matches.len() == 1 {
                Ok(matches[0] as u32)
            } else {
                bail!(r#"Invalid month "{month}""#)
            }
        }
        Ok(value) => {
            if value < 1 || value > 12 {
                bail!(r#"month "{value}" not in the range 1 through 12"#)
            }
            Ok(value)
        }
    }
}

fn format_year(year: i32, today: NaiveDate) -> Vec<String> {
    let mut combined_lines: Vec<String> = Vec::new();

    // Year
    combined_lines.push(format!("{year:>32}"));

    let months: Vec<_> = (1..=12)
        .map(|month| format_month(year, month, false, today))
        .collect();

    for (quarter, chunk) in months.chunks(3).enumerate() {
        if let [month1, month2, month3] = chunk {
            for lines in izip!(month1, month2, month3) {
                combined_lines.push(format!("{}{}{}", lines.0, lines.1, lines.2));
            }

            if quarter < 3 {
                combined_lines.push("".to_string());
            }
        }
    }

    combined_lines
}

fn format_month(year: i32, month: u32, print_year: bool, today: NaiveDate) -> Vec<String> {
    let mut lines: Vec<String> = Vec::with_capacity(8);

    // Month and/or year
    let month_name = MONTHS[(month as usize) - 1];
    lines.push(format!("{:^20}  ",
        if print_year {
            format!("{month_name} {year}")
        } else {
            month_name.to_string()
        }
    ));

    // Days of the week
    lines.push(format!("Su Mo Tu We Th Fr Sa  "));

    let mut week_first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let month_last_day = last_day_in_month(year, month);

    // Weeks in the month
    while week_first_day <= month_last_day {
        let sun_offset = week_first_day.weekday().num_days_from_sunday() as usize;

        let days = week_first_day
            .iter_days()
            .take(6 - sun_offset + 1)
            .map(|date| {
                let fmt = if date.month() == month {
                    format!("{:>2}", date.day())
                } else {
                    format!("{:>2}", "".to_string())
                };

                if date == today {
                    Style::new().reverse().paint(fmt).to_string()
                } else {
                    fmt
                }
            })
            .collect::<Vec<_>>();

        // Insert leading spaces
        let leading = format!("{:leading$}", "", leading = sun_offset * CHARS_PER_DAY);
        lines.push(format!("{leading}{}  ", days.join(" ")));

        // Move to the following week
        week_first_day = week_first_day
            .checked_add_days(Days::new(6 - sun_offset as u64 + 1))
            .unwrap()
    }

    while lines.len() < 8 {
        lines.push(" ".repeat(LINE_LEN));
    }

    lines
}

fn last_day_in_month(year: i32, month: u32) -> NaiveDate {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };

    next_month.checked_sub_days(Days::new(1)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::{format_month, last_day_in_month, parse_month};
    use chrono::NaiveDate;

    #[test]
    fn test_parse_month() {
        let res = parse_month("1".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("12".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 12u32);

        let res = parse_month("jan".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("0".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"month "0" not in the range 1 through 12"#
        );

        let res = parse_month("13".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"month "13" not in the range 1 through 12"#
        );

        let res = parse_month("foo".to_string());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), r#"Invalid month "foo""#);
    }

    #[test]
    fn test_format_month() {
        let today = NaiveDate::from_ymd_opt(0, 1, 1).unwrap();
        let leap_february = vec![
            "   February 2020      ",
            "Su Mo Tu We Th Fr Sa  ",
            "                   1  ",
            " 2  3  4  5  6  7  8  ",
            " 9 10 11 12 13 14 15  ",
            "16 17 18 19 20 21 22  ",
            "23 24 25 26 27 28 29  ",
            "                      ",
        ];
        assert_eq!(format_month(2020, 2, true, today), leap_february);

        let may = vec![
            "        May           ",
            "Su Mo Tu We Th Fr Sa  ",
            "                1  2  ",
            " 3  4  5  6  7  8  9  ",
            "10 11 12 13 14 15 16  ",
            "17 18 19 20 21 22 23  ",
            "24 25 26 27 28 29 30  ",
            "31                    ",
        ];
        assert_eq!(format_month(2020, 5, false, today), may);

        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1  2  3  ",
            " 4  5  6 \u{1b}[7m 7\u{1b}[0m  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd_opt(2021, 4, 7).unwrap();
        assert_eq!(format_month(2021, 4, true, today), april_hl);
    }

    #[test]
    fn test_last_day_in_month() {
        assert_eq!(
            last_day_in_month(2020, 1),
            NaiveDate::from_ymd_opt(2020, 1, 31).unwrap()
        );
        assert_eq!(
            last_day_in_month(2020, 2),
            NaiveDate::from_ymd_opt(2020, 2, 29).unwrap()
        );
        assert_eq!(
            last_day_in_month(2020, 4),
            NaiveDate::from_ymd_opt(2020, 4, 30).unwrap()
        );
    }
}
