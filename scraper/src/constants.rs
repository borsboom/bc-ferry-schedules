use crate::imports::*;

// These are holiday mondays that are not explicitly included in a source schedule annotation
pub const EXTRA_HOLIDAY_MONDAYS: &[Date] = &[date!(2022 - 10 - 10)];

pub static HTML_ERROR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"An error occurred, try again").expect("HTML error regex to parse"));
