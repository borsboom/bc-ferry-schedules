use crate::imports::*;
use crate::types::*;
use ::scraper::ElementRef;
use ::std::collections::hash_map::DefaultHasher;
use ::std::hash::Hasher;

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
    element_texts(elem).join(" ")
}

pub fn element_texts(elem: &ElementRef) -> Vec<String> {
    elem.text().map(|s| regex!(r"\s+").replace_all(s, " ").trim().to_string()).filter(|s| !s.is_empty()).collect()
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

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn parse_schedule_time(text: &str) -> Result<NaiveTime> {
    const TIME_FORMAT: &str = "%l:%M %p";
    const ALT_TIME_FORMAT: &str = "%l;%M %p";
    if let Ok(time) = NaiveTime::parse_from_str(text, TIME_FORMAT) {
        Ok(time)
    } else {
        let time = NaiveTime::parse_from_str(text, ALT_TIME_FORMAT).with_context(|| {
            format!("Invalid schedule time (expect format {:?} or {:?}): {:?}", TIME_FORMAT, ALT_TIME_FORMAT, text)
        })?;
        Ok(time)
    }
}

pub fn parse_schedule_stops<T: AsRef<str>, I: IntoIterator<Item = T>>(texts: I) -> Result<Vec<Stop>> {
    texts
        .into_iter()
        .filter_map(|s| match s.as_ref().trim() {
            "Non-stop" | "non-stop" => None,
            t => Some(Stop::parse_schedule_text(t)),
        })
        .collect()
}
