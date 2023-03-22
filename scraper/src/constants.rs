use crate::imports::*;

pub static HTML_ERROR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"An error occurred, try again").expect("Expect HTML error regex to parse"));

pub static DISABLED_TERMINAL_PAIRS: Lazy<HashSet<TerminalPair>> = Lazy::new(|| HashSet::from_iter([]));
