use scraper::ElementRef;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use crate::imports::*;
use crate::macros::*;

pub fn element_text(elem: &ElementRef) -> String {
    element_texts(elem).join(" ")
}

pub fn element_texts(elem: &ElementRef) -> Vec<String> {
    elem.text().map(|s| regex!(r"\s+").replace_all(s, " ").trim().to_string()).filter(|s| !s.is_empty()).collect()
}

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn parse_schedule_time(text: &str) -> Result<Time> {
    const SCHEDULE_TIME_FORMATS: &[&TimeFormat] = &[
        format_description!("[hour repr:12 padding:none]:[minute] [period case:lower case_sensitive:false]"),
        format_description!("[hour repr:12 padding:none]:[minute][period case:lower case_sensitive:false]"),
        format_description!("[hour repr:12 padding:none];[minute] [period case:lower case_sensitive:false]"),
        format_description!("[hour repr:12 padding:none].[minute] [period case:lower case_sensitive:false]"),
    ];
    for format in SCHEDULE_TIME_FORMATS {
        if let Ok(time) = Time::parse(text, format) {
            return Ok(time);
        }
    }
    bail!("Invalid schedule time: {:?}", text);
}

fn terminal_code_from_schedule_stop_text(stop_text: &str) -> Result<TerminalCode> {
    let stop_text = stop_text.to_lowercase();
    match &stop_text[..] {
        "mayne" | "mayne island (village bay)" => Ok(TerminalCode::PVB),
        "pender" | "pender island (otter bay)" => Ok(TerminalCode::POB),
        "saturna" | "saturna island (lyall harbour)" => Ok(TerminalCode::PST),
        "galiano" | "galiano island (sturdies bay)" => Ok(TerminalCode::PSB),
        "salt spring" | "salt spring island (long harbour)" => Ok(TerminalCode::PLH),
        "victoria (swartz bay)" => Ok(TerminalCode::PLH),
        _ => Err(anyhow!("Unknown schedule stop name: {:?}", stop_text)),
    }
}

fn parse_stop_schedule_text(stop_text: &str) -> Result<Stop> {
    let inner = || {
        if let Some(captures) = regex!(r"(?i)^transfer( at)? (.*)$").captures(stop_text) {
            Ok(Stop { type_: StopType::Transfer, terminal: terminal_code_from_schedule_stop_text(&captures[2])? })
                as Result<_>
        } else {
            let stop_text = &regex!(r"(?i)^(stop( at)? )?(.*)$").captures(stop_text).expect("stop text to match")[3];
            Ok(Stop { type_: StopType::Stop, terminal: terminal_code_from_schedule_stop_text(stop_text)? })
        }
    };
    inner().with_context(|| format!("Failed to parse schedule stop: {:?}", stop_text))
}

pub fn parse_schedule_stops<T: AsRef<str>, I: IntoIterator<Item = T>>(texts: I) -> Result<Vec<Stop>> {
    texts
        .into_iter()
        .filter_map(|s| match s.as_ref().trim() {
            "Non-stop" | "non-stop" => None,
            t => Some(parse_stop_schedule_text(t)),
        })
        .collect()
}

pub fn should_scrape_schedule_date(schedule_date_range: DateRange, today: Date, restrict_date: Option<Date>) -> bool {
    schedule_date_range.to >= today
        && restrict_date.map(|date| schedule_date_range.includes_date_inclusive(date)).unwrap_or(true)
}
