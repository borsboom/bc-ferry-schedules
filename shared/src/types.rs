use crate::imports::*;

pub type TimeFormat = [time::format_description::FormatItem<'static>];

#[derive(
    Copy, Clone, Debug, Deserialize, Display, EnumString, Eq, EnumIter, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum TerminalCode {
    PSB, // Galiano Island (Sturdies Bay)
    PVB, // Mayne Island (Village Bay)
    POB, // Pender Island (Otter Bay)
    PLH, // Salt Spring Island (Long Harbour)
    PST, // Saturna Island (Lyall Harbour)
    TSA, // Vancouver (Tsawwassen)
    SWB, // Victoria (Swartz Bay)
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TerminalCodePair {
    pub from: TerminalCode,
    pub to: TerminalCode,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum StopType {
    Stop,
    Transfer,
    Thrufare,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Stop {
    pub type_: StopType,
    pub terminal: TerminalCode,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Sailing {
    pub depart_time: Time,
    pub arrive_time: Time,
    pub stops: Vec<Stop>,
}

#[derive(Clone, Debug)]
struct DateDaysIterator {
    date: Option<Date>,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct DateRange {
    pub from: Date,
    pub to: Date,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DateRestriction {
    All,
    Only(HashSet<Date>),
    Except(HashSet<Date>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScheduleItem {
    pub sailing: Sailing,
    pub weekdays: HashMap<Weekday, DateRestriction>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub notes: HashMap<Cow<'static, str>, DateRestriction>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Schedule {
    pub terminal_pair: TerminalCodePair,
    pub date_range: DateRange,
    pub items: Vec<ScheduleItem>,
    pub source_url: String,
    pub refreshed_at: OffsetDateTime,
}

impl TerminalCode {
    pub fn is_gulf_island(&self) -> bool {
        match *self {
            TerminalCode::PLH => true,
            TerminalCode::POB => true,
            TerminalCode::PSB => true,
            TerminalCode::PST => true,
            TerminalCode::PVB => true,
            TerminalCode::SWB => false,
            TerminalCode::TSA => false,
        }
    }

    pub fn long_location_name(&self) -> &'static str {
        match *self {
            TerminalCode::PLH => "Salt Spring Island",
            TerminalCode::POB => "Pender Island",
            TerminalCode::PSB => "Galiano Island",
            TerminalCode::PST => "Saturna Island",
            TerminalCode::PVB => "Mayne Island",
            TerminalCode::SWB => "Victoria",
            TerminalCode::TSA => "Vancouver",
        }
    }

    pub fn short_location_name(&self) -> &'static str {
        match *self {
            TerminalCode::PLH => "Salt Spring",
            TerminalCode::POB => "Pender",
            TerminalCode::PSB => "Galiano",
            TerminalCode::PST => "Saturna",
            TerminalCode::PVB => "Mayne",
            TerminalCode::SWB => "Victoria",
            TerminalCode::TSA => "Vancouver",
        }
    }

    pub fn terminal_name(&self) -> &'static str {
        match *self {
            TerminalCode::PLH => "Long Harbour",
            TerminalCode::POB => "Otter Bay",
            TerminalCode::PSB => "Sturdies Bay",
            TerminalCode::PST => "Lyall Harbour",
            TerminalCode::PVB => "Village Bay",
            TerminalCode::SWB => "Swartz Bay",
            TerminalCode::TSA => "Tsawwassen",
        }
    }
}

impl TerminalCodePair {
    pub fn parse_schedule_code_pair(code_pair: &str) -> Result<TerminalCodePair> {
        let inner = || {
            let parts: Vec<_> = code_pair.split('-').collect();
            if parts.len() != 2 {
                bail!("Expect exactly two parts");
            }
            let from = parts[0].parse().with_context(|| format!("Invalid first part: {:?}", parts[0]))?;
            let to = parts[1].parse().with_context(|| format!("Invalid second part: {:?}", parts[1]))?;
            Ok(TerminalCodePair { from, to })
        };
        inner().with_context(|| format!("Failed to parse terminal code pair: {:?}", code_pair))
    }

    pub fn to_schedule_code_pair(self) -> String {
        format!("{}-{}", self.from, self.to)
    }

    pub fn includes_terminal(&self, terminal: TerminalCode) -> bool {
        self.from == terminal || self.to == terminal
    }

    pub fn is_visible(&self) -> bool {
        self.from != self.to
            && (self.from.is_gulf_island() || self.to.is_gulf_island())
            && !(self.from == TerminalCode::SWB && self.to == TerminalCode::PLH)
    }
}

impl Display for TerminalCodePair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_schedule_code_pair())
    }
}

impl FromStr for TerminalCodePair {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<TerminalCodePair> {
        TerminalCodePair::parse_schedule_code_pair(s)
    }
}

impl Sailing {
    pub fn is_thrufare(&self) -> bool {
        self.stops.iter().any(|s| s.type_ == StopType::Thrufare)
    }
}

impl DateDaysIterator {
    pub fn new(date: Date) -> DateDaysIterator {
        DateDaysIterator { date: Some(date) }
    }
}

impl Iterator for DateDaysIterator {
    type Item = Date;
    fn next(&mut self) -> Option<Date> {
        let result = self.date;
        self.date = self.date.and_then(Date::next_day);
        result
    }
}

impl DateRange {
    pub fn iter_days(&self) -> impl Iterator<Item = Date> + '_ {
        DateDaysIterator::new(self.from).take_while(|d| d <= &self.to)
    }

    pub fn includes_date_inclusive(&self, date: Date) -> bool {
        date >= self.from && date <= self.to
    }

    pub fn make_year_within(&self, orig_date: Date) -> Result<Date> {
        const ERROR_DATE_FORMAT: &TimeFormat = format_description!("[month repr:short] [day padding:none]");
        let inner = || {
            let fixed_date = if orig_date < self.from {
                let from_date = Date::from_calendar_date(self.from.year(), orig_date.month(), orig_date.day())?;
                if from_date < self.from {
                    Date::from_calendar_date(self.from.year() + 1, from_date.month(), from_date.day())?
                } else {
                    from_date
                }
            } else if orig_date > self.to {
                let to_date = Date::from_calendar_date(self.to.year(), orig_date.month(), orig_date.day())?;
                if to_date > self.to {
                    Date::from_calendar_date(self.to.year() - 1, to_date.month(), to_date.day())?
                } else {
                    to_date
                }
            } else {
                orig_date
            };
            ensure!(
                self.includes_date_inclusive(fixed_date),
                "{} is not within date range {}",
                fixed_date.format(ERROR_DATE_FORMAT).expect("date within year to format"),
                self,
            );
            Ok(fixed_date)
        };
        inner().with_context(|| {
            format!(
                "Failed to make date {} within range {}",
                orig_date.format(ERROR_DATE_FORMAT).expect("date within year to format"),
                self
            )
        })
    }

    pub fn parse(text: &str, date_format: &TimeFormat, separator: &str) -> Result<DateRange> {
        let inner = || {
            let parts: Vec<_> = text.split(separator).collect();
            if parts.len() != 2 {
                bail!("Expect exactly two parts (expect separator {:?})", separator);
            }
            let from = Date::parse(parts[0], date_format)
                .context(format!("Invalid first part (expect date format {:?}): {:?}", date_format, parts[0]))?;
            let to = Date::parse(parts[1], date_format)
                .context(format!("Invalid second part (expect date format {:?}): {:?}", date_format, parts[1]))?;
            Ok(DateRange { from, to })
        };
        inner().with_context(|| format!("Failed to parse date range: {:?}", text))
    }
}

impl fmt::Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}", self.from, self.to)
    }
}

impl DateRestriction {
    pub fn includes_date(&self, date: Date) -> bool {
        match self {
            DateRestriction::All => true,
            DateRestriction::Except(dates) => !dates.contains(&date),
            DateRestriction::Only(dates) => dates.contains(&date),
        }
    }

    pub fn is_never(&self) -> bool {
        match self {
            DateRestriction::All => false,
            DateRestriction::Except(_) => false,
            DateRestriction::Only(dates) => dates.is_empty(),
        }
    }
    pub fn merge(&mut self, other: &DateRestriction) -> Result<()> {
        match (self, other) {
            (DateRestriction::Except(a), DateRestriction::Except(b)) => a.extend(b),
            (DateRestriction::Only(a), DateRestriction::Only(b)) => a.extend(b),
            (DateRestriction::All, DateRestriction::All) => {}
            (a @ DateRestriction::All, DateRestriction::Except(b)) => *a = DateRestriction::Except(b.clone()),
            (DateRestriction::Except(_), DateRestriction::All) => {}
            (a, b) => bail!("Conflict in date restrictions to merge: {:?} and {:?}", a, b),
        }
        Ok(())
    }

    pub fn merge_map<K: Eq + Hash + Debug>(
        existing_map: &mut HashMap<K, DateRestriction>,
        new_map: HashMap<K, DateRestriction>,
    ) -> Result<()> {
        for (key, new_dr) in new_map {
            if let Some(existing_dr) = existing_map.get_mut(&key) {
                existing_dr
                    .merge(&new_dr)
                    .with_context(|| format!("Failed to merge date restrictions for key: {:?}", key))?;
            } else {
                existing_map.insert(key, new_dr);
            }
        }
        Ok(())
    }
}

impl ScheduleItem {
    pub fn merge_items(items: Vec<ScheduleItem>) -> Result<Vec<ScheduleItem>> {
        let mut map: HashMap<Sailing, ScheduleItem> = HashMap::new();
        for new_item in items {
            if let Some(existing_item) = map.get_mut(&new_item.sailing) {
                DateRestriction::merge_map(&mut existing_item.weekdays, new_item.weekdays)
                    .context("Failed to merge weekdays of schedule items")?;
                DateRestriction::merge_map(&mut existing_item.notes, new_item.notes)
                    .context("Failed to merge notes of schedule items")?;
            } else {
                map.insert(new_item.sailing.clone(), new_item);
            }
        }
        let mut items: Vec<_> = map.into_values().collect();
        items.sort_unstable_by(|a, b| a.sailing.depart_time.cmp(&b.sailing.depart_time));
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_range_make_year_within() -> Result<()> {
        let range = DateRange { from: date!(2021 - 10 - 01), to: date!(2022 - 03 - 31) };
        assert_eq!(range.make_year_within(date!(2021 - 03 - 31))?, date!(2022 - 03 - 31));
        assert_eq!(range.make_year_within(date!(2022 - 10 - 01))?, date!(2021 - 10 - 01));
        assert_eq!(range.make_year_within(date!(2021 - 02 - 12))?, date!(2022 - 02 - 12));
        assert_eq!(range.make_year_within(date!(2022 - 11 - 23))?, date!(2021 - 11 - 23));
        assert!(range.make_year_within(date!(2022 - 04 - 01)).is_err());
        assert!(range.make_year_within(date!(2021 - 09 - 30)).is_err());
        assert!(range.make_year_within(date!(2021 - 07 - 15)).is_err());
        Ok(())
    }

    #[test]
    fn test_date_range_parse() -> Result<()> {
        assert_eq!(
            DateRange::parse(
                "March 31, 2021 - October 1, 2021",
                format_description!("[month repr:long case_sensitive:false] [day padding:none], [year]"),
                " - "
            )?,
            DateRange { from: date!(2021 - 03 - 31), to: date!(2021 - 10 - 01) }
        );
        assert_eq!(
            DateRange::parse("20210331-20211001", format_description!("[year][month][day]"), "-")?,
            DateRange { from: date!(2021 - 03 - 31), to: date!(2021 - 10 - 01) }
        );
        Ok(())
    }

    #[test]
    fn test_date_range_iter_days() -> Result<()> {
        assert_eq!(
            DateRange { from: date!(2021 - 03 - 30), to: date!(2021 - 04 - 01) }.iter_days().collect::<Vec<_>>(),
            vec![date!(2021 - 03 - 30), date!(2021 - 03 - 31), date!(2021 - 04 - 01)]
        );
        Ok(())
    }
}
