use crate::imports::*;
use crate::utils::*;
use ::clap::Args;
use ::std::fmt;

#[derive(Args, Debug)]
pub struct Options {
    /// Force to process even if source data is unchanged
    #[clap(short, long)]
    pub force: bool,

    /// Ignore the cache and always re-download the source data
    #[clap(short, long)]
    pub ignore_cache: bool,

    /// Only process schedules for specified terminal pair
    #[clap(short, long, value_name = "FROM-TO")]
    pub terminals: Option<TerminalCodePair>,

    /// Only process schedules whose date range includes this date
    #[clap(short, long, value_name = "YYYY-MM-DD")]
    pub date: Option<NaiveDate>,
}

#[derive(Copy, Clone, Debug, EnumString, Eq, Display, Hash, Ord, PartialEq, PartialOrd)]
pub enum TerminalCode {
    #[strum(serialize = "PLH")]
    Plh,
    #[strum(serialize = "POB")]
    Pob,
    #[strum(serialize = "PSB")]
    Psb,
    #[strum(serialize = "PST")]
    Pst,
    #[strum(serialize = "PVB")]
    Pvb,
    #[strum(serialize = "SWB")]
    Swb,
    #[strum(serialize = "TSA")]
    Tsa,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct TerminalCodePair {
    pub from: TerminalCode,
    pub to: TerminalCode,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum StopType {
    Stop,
    Transfer,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Stop {
    pub type_: StopType,
    pub terminal: TerminalCode,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sailing {
    pub depart_time: NaiveTime,
    pub arrive_time: NaiveTime,
    pub stops: Vec<Stop>,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum RouteGroup {
    SaltSpringAndOuterGulfIslands,
}

#[derive(Debug)]
pub enum DateRestriction {
    Only(HashSet<NaiveDate>),
    Except(HashSet<NaiveDate>),
}

#[derive(Debug)]
pub struct ScheduleItem {
    pub sailing: Sailing,
    pub weekdays: HashMap<Weekday, DateRestriction>,
    pub notes: HashMap<&'static str, DateRestriction>,
}

#[derive(Debug)]
pub struct Schedule {
    pub terminal_pair: TerminalCodePair,
    pub effective_date_range: DateRange,
    pub items: Vec<ScheduleItem>,
    pub source_url: String,
    pub route_group: RouteGroup,
    pub reservable: bool,
}

impl TerminalCode {
    pub fn from_schedule_stop_text(stop_text: &str) -> Result<TerminalCode> {
        let stop_text = stop_text.to_lowercase();
        match &stop_text[..] {
            "mayne" | "mayne island (village bay)" => Ok(TerminalCode::Pvb),
            "pender" | "pender island (otter bay)" => Ok(TerminalCode::Pob),
            "saturna" | "saturna island (lyall harbour)" => Ok(TerminalCode::Pst),
            "galiano" | "galiano island (sturdies bay)" => Ok(TerminalCode::Psb),
            "salt spring" | "salt spring island (long harbour)" => Ok(TerminalCode::Plh),
            _ => Err(anyhow!("Unknown schedule stop name: {:?}", stop_text)),
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
}

impl Display for TerminalCodePair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_schedule_code_pair())
    }
}

impl FromStr for TerminalCodePair {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<TerminalCodePair> {
        TerminalCodePair::parse_schedule_code_pair(s)
    }
}

impl Stop {
    pub fn parse_schedule_text(stop_text: &str) -> Result<Stop> {
        let inner = || {
            if let Some(captures) = regex!(r"(?i)^transfer( at)? (.*)$").captures(stop_text) {
                Ok(Stop { type_: StopType::Transfer, terminal: TerminalCode::from_schedule_stop_text(&captures[2])? })
                    as Result<_>
            } else {
                let stop_text = &regex!(r"(?i)^(stop( at)? )?(.*)$").captures(stop_text).unwrap()[3];
                Ok(Stop { type_: StopType::Stop, terminal: TerminalCode::from_schedule_stop_text(stop_text)? })
            }
        };
        inner().with_context(|| format!("Failed to parse schedule stop: {:?}", stop_text))
    }
}

impl DateRange {
    pub fn iter_days(&self) -> impl Iterator<Item = NaiveDate> + '_ {
        self.from.iter_days().take_while(|d| d <= &self.to)
    }

    pub fn date_within_inclusive(&self, date: NaiveDate) -> bool {
        date >= self.from && date <= self.to
    }

    pub fn make_year_within(&self, orig_date: NaiveDate) -> Result<NaiveDate> {
        let fixed_date = if orig_date < self.from {
            let from_date = date(self.from.year(), orig_date.month(), orig_date.day());
            if from_date < self.from {
                date(self.from.year() + 1, from_date.month(), from_date.day())
            } else {
                from_date
            }
        } else if orig_date > self.to {
            let to_date = date(self.to.year(), orig_date.month(), orig_date.day());
            if to_date > self.to {
                date(self.to.year() - 1, to_date.month(), to_date.day())
            } else {
                to_date
            }
        } else {
            orig_date
        };
        ensure!(
            self.date_within_inclusive(fixed_date),
            "{} is not within date range {}",
            fixed_date.format("%b %d"),
            self,
        );
        Ok(fixed_date)
    }

    fn parse(text: &str, date_format: &str, separator: &str) -> Result<DateRange> {
        let inner = || {
            let parts: Vec<_> = text.split(separator).collect();
            if parts.len() != 2 {
                bail!("Expect exactly two parts (expect separator separator {:?})", separator);
            }
            let from = NaiveDate::parse_from_str(parts[0], date_format)
                .context(format!("Invalid first part (expect date format {:?}): {:?}", date_format, parts[0]))?;
            let to = NaiveDate::parse_from_str(parts[1], date_format)
                .context(format!("Invalid second part (expect date format {:?}): {:?}", date_format, parts[1]))?;
            Ok(DateRange { from, to })
        };
        inner().with_context(|| format!("Failed to parse date range: {:?}", text))
    }

    pub fn parse_schedule_html_text(text: &str) -> Result<DateRange> {
        DateRange::parse(text, "%B %e, %Y", " - ")
            .with_context(|| format!("Failed to parse schedule HTML date range: {:?}", text))
    }

    pub fn parse_schedule_query_value(text: &str) -> Result<DateRange> {
        DateRange::parse(text, "%Y%m%d", "-")
            .with_context(|| format!("Failed to parse schedule query date range: {:?}", text))
    }
}

impl fmt::Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}", self.from, self.to)
    }
}

impl DateRestriction {
    pub fn includes_date(&self, date: NaiveDate) -> bool {
        match self {
            DateRestriction::Except(dates) => !dates.contains(&date),
            DateRestriction::Only(dates) => dates.contains(&date),
        }
    }

    pub fn is_never(&self) -> bool {
        match self {
            DateRestriction::Except(_) => false,
            DateRestriction::Only(dates) => dates.is_empty(),
        }
    }

    pub fn merge(&mut self, other: &DateRestriction) -> Result<()> {
        match (self, other) {
            (DateRestriction::Except(self_except), DateRestriction::Except(other_except)) => {
                self_except.extend(other_except)
            }
            (DateRestriction::Only(self_only), DateRestriction::Only(other_only)) => self_only.extend(other_only),
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
        items.sort_unstable_by(|a, b| a.sailing.depart_time.partial_cmp(&b.sailing.depart_time).unwrap());
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_range_make_year_within() -> Result<()> {
        let range = DateRange { from: date(2021, 10, 1), to: date(2022, 3, 31) };
        assert_eq!(range.make_year_within(date(2021, 3, 31))?, date(2022, 3, 31));
        assert_eq!(range.make_year_within(date(2022, 10, 1))?, date(2021, 10, 1));
        assert_eq!(range.make_year_within(date(2021, 2, 12))?, date(2022, 2, 12));
        assert_eq!(range.make_year_within(date(2022, 11, 23))?, date(2021, 11, 23));
        assert!(range.make_year_within(date(2022, 4, 1)).is_err());
        assert!(range.make_year_within(date(2021, 9, 30)).is_err());
        assert!(range.make_year_within(date(2021, 7, 15)).is_err());
        Ok(())
    }
}
