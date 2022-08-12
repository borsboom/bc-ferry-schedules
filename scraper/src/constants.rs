use crate::imports::*;

pub static HTML_ERROR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"An error occurred, try again").expect("HTML error regex to parse"));
