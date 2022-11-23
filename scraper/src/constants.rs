use crate::imports::*;

// These are holiday mondays that are not explicitly included in a source schedule annotation
pub const EXTRA_HOLIDAY_MONDAYS: &[Date] = &[date!(2022 - 10 - 10)];

pub static HTML_ERROR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"An error occurred, try again").expect("Expect HTML error regex to parse"));

pub const FOOT_PASSENGERS_ONLY_NOTE: &str = "Foot passengers only";

pub const DANGEROUS_GOODS_SAILING_NOTE: &str = "Dangerous goods sailing only, no other passengers permitted";
