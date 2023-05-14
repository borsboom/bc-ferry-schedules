use crate::constants::*;
use crate::imports::*;

pub type TimeFormat = [time::format_description::FormatItem<'static>];

#[derive(
    Copy, Clone, Debug, Deserialize, Display, EnumString, Eq, EnumIter, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum Area {
    Brentwood,
    Chemainus,
    Crofton,
    // Gulf Islands terminals have aliases for backward compatibility when parsing URL query
    #[serde(alias = "PSB")]
    Galiano,
    #[serde(alias = "PVB")]
    Mayne,
    MillBay,
    #[serde(alias = "POB")]
    Pender,
    Penelakut,
    #[serde(alias = "PLH")]
    SaltSpring,
    #[serde(alias = "PST")]
    Saturna,
    Thetis,
    #[serde(alias = "TSA")]
    Vancouver,
    #[serde(alias = "SWB")]
    Victoria,
}

#[derive(
    Copy, Clone, Debug, Deserialize, Display, EnumString, Eq, EnumIter, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum Terminal {
    BTW, // Brentwood Bay
    CFT, // Crofton
    CHM, // Chemainus
    FUL, // Salt Spring Island (Fulford Harbour)
    MIL, // Mill Bay
    PEN, // Penelakut Island (Telegraph Harbour)
    PLH, // Salt Spring Island (Long Harbour)
    POB, // Pender Island (Otter Bay)
    PSB, // Galiano Island (Sturdies Bay)
    PST, // Saturna Island (Lyall Harbour)
    PVB, // Mayne Island (Village Bay)
    SWB, // Victoria (Swartz Bay)
    THT, // Thetis Island (Preedy Harbour)
    TSA, // Vancouver (Tsawwassen)
    VES, // Salt Spring Island (Vesuvius Bay)
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct AreaPair {
    pub from: Area,
    pub to: Area,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TerminalPair {
    pub from: Terminal,
    pub to: Terminal,
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
    pub terminal: Terminal,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Alert {
    pub message: String,
    pub level: AlertLevel,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Schedule {
    pub terminal_pair: TerminalPair,
    pub date_range: DateRange,
    pub items: Vec<ScheduleItem>,
    pub source_url: String,
    pub refreshed_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub alerts: Vec<Alert>,
}

impl Area {
    pub fn long_name(&self) -> &'static str {
        match *self {
            Area::Brentwood => "Brentwood",
            Area::Chemainus => "Chemainus",
            Area::Crofton => "Crofton",
            Area::Galiano => "Galiano Island",
            Area::Mayne => "Mayne Island",
            Area::MillBay => "Mill Bay",
            Area::Pender => "Pender Island",
            Area::Penelakut => "Penelakut Island",
            Area::SaltSpring => "Salt Spring Island",
            Area::Saturna => "Saturna Island",
            Area::Thetis => "Thetis Island",
            Area::Vancouver => "Vancouver",
            Area::Victoria => "Victoria",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match *self {
            Area::Brentwood => "Brentwood",
            Area::Chemainus => "Chemainus",
            Area::Crofton => "Crofton",
            Area::Galiano => "Galiano",
            Area::Mayne => "Mayne",
            Area::MillBay => "Mill Bay",
            Area::Pender => "Pender",
            Area::Penelakut => "Penelakut",
            Area::SaltSpring => "Salt Spring",
            Area::Saturna => "Saturna",
            Area::Thetis => "Thetis",
            Area::Vancouver => "Vancouver",
            Area::Victoria => "Victoria",
        }
    }

    pub fn includes_terminal(&self, terminal: Terminal) -> bool {
        self.includes_any_terminal(iter::once(terminal))
    }

    pub fn includes_any_terminal<I: IntoIterator<Item = Terminal>>(&self, terminals: I) -> bool {
        AREA_TERMINALS.get(self).map(|v| terminals.into_iter().any(|t| v.contains(&t))).unwrap_or(false)
    }
}

impl Terminal {
    pub fn name(&self) -> &'static str {
        match *self {
            Terminal::BTW => "Brentwood Bay",
            Terminal::CFT => "Crofton",
            Terminal::CHM => "Chemainus",
            Terminal::FUL => "Fulford Harbour",
            Terminal::MIL => "Mill Bay",
            Terminal::PEN => "Telegraph Harbour",
            Terminal::PLH => "Long Harbour",
            Terminal::POB => "Otter Bay",
            Terminal::PSB => "Sturdies Bay",
            Terminal::PST => "Lyall Harbour",
            Terminal::PVB => "Village Bay",
            Terminal::SWB => "Swartz Bay",
            Terminal::THT => "Preedy Harbour",
            Terminal::TSA => "Tsawwassen",
            Terminal::VES => "Vesuvius Bay",
        }
    }

    pub fn area(&self) -> Area {
        match *self {
            Terminal::BTW => Area::Brentwood,
            Terminal::CFT => Area::Crofton,
            Terminal::CHM => Area::Chemainus,
            Terminal::FUL => Area::SaltSpring,
            Terminal::MIL => Area::MillBay,
            Terminal::PEN => Area::Penelakut,
            Terminal::PLH => Area::SaltSpring,
            Terminal::POB => Area::Pender,
            Terminal::PSB => Area::Galiano,
            Terminal::PST => Area::Saturna,
            Terminal::PVB => Area::Mayne,
            Terminal::SWB => Area::Victoria,
            Terminal::THT => Area::Thetis,
            Terminal::TSA => Area::Vancouver,
            Terminal::VES => Area::SaltSpring,
        }
    }

    pub fn combinations(terminals: &[Terminal]) -> impl Iterator<Item = TerminalPair> + '_ {
        terminals
            .iter()
            .combinations(2)
            .flat_map(|v| [TerminalPair { from: *v[0], to: *v[1] }, TerminalPair { from: *v[1], to: *v[0] }])
    }
}

impl AreaPair {
    pub fn swapped(&self) -> AreaPair {
        AreaPair { from: self.to, to: self.from }
    }

    pub fn includes_terminal(&self, terminal: Terminal) -> bool {
        self.from.includes_terminal(terminal) || self.to.includes_terminal(terminal)
    }

    pub fn includes_any_terminal(&self, terminals: &HashSet<Terminal>) -> bool {
        self.from.includes_any_terminal(terminals.iter().cloned())
            || self.to.includes_any_terminal(terminals.iter().cloned())
    }

    pub fn is_reservable(&self) -> bool {
        self.includes_any_terminal(&*ROUTE_5_AND_9_GULF_ISLAND_TERMINALS) && self.includes_terminal(Terminal::TSA)
    }
}

impl TerminalPair {
    pub fn parse_schedule_code_pair(code_pair: &str) -> Result<TerminalPair> {
        let inner = || {
            let parts: Vec<_> = code_pair.split('-').collect();
            if parts.len() != 2 {
                bail!("Expect exactly two parts");
            }
            let from = parts[0].parse().with_context(|| format!("Invalid first part: {:?}", parts[0]))?;
            let to = parts[1].parse().with_context(|| format!("Invalid second part: {:?}", parts[1]))?;
            Ok(TerminalPair { from, to })
        };
        inner().with_context(|| format!("Failed to parse terminal code pair: {:?}", code_pair))
    }

    pub fn to_schedule_code_pair(self) -> String {
        format!("{}-{}", self.from, self.to)
    }

    pub fn includes_terminal(&self, terminal: Terminal) -> bool {
        self.from == terminal || self.to == terminal
    }

    pub fn includes_any_terminal(&self, terminals: &HashSet<Terminal>) -> bool {
        terminals.contains(&self.from) || terminals.contains(&self.to)
    }

    pub fn swapped(&self) -> TerminalPair {
        TerminalPair { from: self.to, to: self.from }
    }

    pub fn area_pair(&self) -> AreaPair {
        AreaPair { from: self.from.area(), to: self.to.area() }
    }
}

impl Display for TerminalPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_schedule_code_pair())
    }
}

impl FromStr for TerminalPair {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<TerminalPair> {
        TerminalPair::parse_schedule_code_pair(s)
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

    pub fn parse_date_within(&self, text: &str) -> Result<Option<Date>> {
        let inner = || {
            // We use year 2020 since it is a leap year, so Feb 29 will parse successfully.
            let text_with_year = format!("{} {}", text, 2020);
            let parsed_date = Date::parse(
                &text_with_year,
                format_description!("[day padding:none] [month repr:short case_sensitive:false] [year]"),
            )
            .or_else(|_| {
                Date::parse(
                    &text_with_year,
                    format_description!("[month repr:short case_sensitive:false] [day padding:none] [year]"),
                )
            })?;
            match Date::from_calendar_date(self.from.year(), parsed_date.month(), parsed_date.day()) {
                Ok(from_year_date) if self.includes_date_inclusive(from_year_date) => Ok(Some(from_year_date)),
                _ if self.from.year() == self.to.year() => Ok(None) as Result<_>,
                _ => match Date::from_calendar_date(self.to.year(), parsed_date.month(), parsed_date.day()) {
                    Ok(to_year_date) if self.includes_date_inclusive(to_year_date) => {
                        Ok(Some(to_year_date)) as Result<_>
                    }
                    _ => Ok(None) as Result<_>,
                },
            }
        };
        inner().with_context(|| format!("Failed to parse date within range {}: {:?}", self, text))
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
        let range = DateRange { from: date!(2023 - 10 - 01), to: date!(2024 - 03 - 31) };
        assert_eq!(range.parse_date_within("Mar 31")?, Some(date!(2024 - 03 - 31)));
        assert_eq!(range.parse_date_within("Oct 01")?, Some(date!(2023 - 10 - 01)));
        assert_eq!(range.parse_date_within("Feb 29")?, Some(date!(2024 - 02 - 29)));
        assert_eq!(range.parse_date_within("Nov 23")?, Some(date!(2023 - 11 - 23)));
        assert!(range.parse_date_within("Apr 01")?.is_none());
        assert!(range.parse_date_within("Sep 30")?.is_none());
        assert!(range.parse_date_within("Jul 15")?.is_none());
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

    #[test]
    fn test_terminal_combinations() -> Result<()> {
        assert_eq!(
            Terminal::combinations(&[Terminal::CHM, Terminal::THT, Terminal::PEN]).collect::<HashSet<_>>(),
            HashSet::from([
                TerminalPair { from: Terminal::CHM, to: Terminal::PEN },
                TerminalPair { from: Terminal::CHM, to: Terminal::THT },
                TerminalPair { from: Terminal::PEN, to: Terminal::CHM },
                TerminalPair { from: Terminal::PEN, to: Terminal::THT },
                TerminalPair { from: Terminal::THT, to: Terminal::CHM },
                TerminalPair { from: Terminal::THT, to: Terminal::PEN },
            ])
        );
        Ok(())
    }
}
