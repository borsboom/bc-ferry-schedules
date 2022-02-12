use crate::imports::*;
use ::scraper::ElementRef;

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

pub fn element_text(elem: &ElementRef) -> String {
    element_texts(elem).join("")
}

pub fn element_texts(elem: &ElementRef) -> Vec<String> {
    elem.text().map(|s| s.trim().replace("\u{a0}", " ")).filter(|s| !s.is_empty()).collect::<Vec<_>>()
}

pub fn format_iso_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn format_hours_minutes(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

pub fn now_pacific() -> DateTime<Tz> {
    Utc::now().with_timezone(&Pacific)
}

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd(y, m, d)
}
