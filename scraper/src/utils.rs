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

pub fn parse_duration(duration_text: &str) -> Result<Duration> {
    let inner = || {
        let duration_captures = regex!(r"^((\d+)h)? ?((\d+)m)?$")
            .captures(duration_text)
            .ok_or_else(|| anyhow!("Invalid duration format: {:?}", duration_text))?;
        let duration = match (duration_captures.get(2), duration_captures.get(4)) {
            (None, None) => bail!("Expect minutes and/or hours in duration"),
            (hours_text, minutes_text) => Duration::minutes(
                hours_text
                    .map(|m| m.as_str().parse::<i64>().expect("Expect duration hours to parse to integer"))
                    .unwrap_or(0)
                    * 60
                    + minutes_text
                        .map(|m| m.as_str().parse::<i64>().expect("Expect duration minutes to parse to integer"))
                        .unwrap_or(0),
            ),
        };
        Ok(duration) as Result<_>
    };
    inner().with_context(|| format!("Failed to parse duration: {:?}", duration_text))
}

pub fn parse_arrive_time_or_duration(depart_time: Time, text: &str) -> Result<Time> {
    parse_schedule_time(text)
        .or_else(|time_err| parse_duration(text).map(|dur| depart_time + dur).context(time_err))
        .with_context(|| format!("Failed to parse arrive time or duration: {:?}", text))
}

fn terminal_from_schedule_stop_text(stop_text: &str) -> Result<Terminal> {
    let stop_text = stop_text.to_lowercase();
    match &stop_text[..] {
        "chemainus" => Ok(Terminal::CHM),
        "galiano" | "galiano island (sturdies bay)" => Ok(Terminal::PSB),
        "mayne"
        | "mayne island (village bay)"
        | "mayne island (village bay"
        | "mayne island {village bay)"
        | "mayne island (village bay)except on oct 9" => Ok(Terminal::PVB),
        "pender" | "pender island (otter bay)" => Ok(Terminal::POB),
        "penelakut island (telegraph harbour)" => Ok(Terminal::PEN),
        "salt spring" | "salt spring island (long harbour)" => Ok(Terminal::PLH),
        "saturna" | "saturna island (lyall harbour)" | "saturna island (lyall harbour" => Ok(Terminal::PST),
        "thetis island (preedy harbour)" => Ok(Terminal::THT),
        "victoria (swartz bay)" | "swartz bay" => Ok(Terminal::SWB),
        _ => Err(anyhow!("Unknown schedule stop name: {:?}", stop_text)),
    }
}

fn parse_stop_schedule_text(stop_text: &str) -> Result<Stop> {
    let inner = || {
        if let Some(captures) = regex!(r"(?i)^(Transfer )?transfer( at)? (.*)$").captures(stop_text) {
            Ok(Stop { type_: StopType::Transfer, terminal: terminal_from_schedule_stop_text(&captures[3])? })
                as Result<_>
        } else if let Some(captures) = regex!(r"(?i)^thru fare( at)? (.*)$").captures(stop_text) {
            Ok(Stop { type_: StopType::Thrufare, terminal: terminal_from_schedule_stop_text(&captures[2])? })
                as Result<_>
        } else {
            let stop_text =
                &regex!(r"(?i)^(Stop )?(stop( at)? )?(.*)$").captures(stop_text).expect("Expect stop text to match")[4];
            Ok(Stop { type_: StopType::Stop, terminal: terminal_from_schedule_stop_text(stop_text)? })
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

pub fn parse_weekday(text: &str) -> Result<Weekday> {
    match text {
        "mon" | "Mondays" | "MONDAY" => Ok(Weekday::Monday),
        "tue" | "Tuesdays" | "TUESDAY" => Ok(Weekday::Tuesday),
        "wed" | "Wednesdays" | "WEDNESDAY" => Ok(Weekday::Wednesday),
        "thu" | "Thursdays" | "THURSDAY" => Ok(Weekday::Thursday),
        "fri" | "Fridays" | "FRIDAY" => Ok(Weekday::Friday),
        "sat" | "Saturdays" | "SATURDAY" => Ok(Weekday::Saturday),
        "sun" | "Sundays" | "SUNDAY" => Ok(Weekday::Sunday),
        _ => bail!("Unrecognized day text: {:?}", text),
    }
}
