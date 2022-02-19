use crate::imports::*;

pub static IGNORE_HTML_CHANGES_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"ACC\.config\.CSRFToken.*").unwrap());
