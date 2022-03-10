macro_rules! regex {
    ($regex:literal $(,)?) => {{
        static REGEX: OnceCell<Regex> = OnceCell::new();
        REGEX.get_or_init(|| Regex::new($regex).unwrap())
    }};
}
pub(crate) use regex;

macro_rules! selector {
    ($selector:literal $(,)?) => {{
        static SELECTOR: OnceCell<Selector> = OnceCell::new();
        SELECTOR.get_or_init(|| Selector::parse($selector).unwrap())
    }};
}
pub(crate) use selector;
