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
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Terminal {
    GalianoIslandSturdiesBay,
    MayneIslandVillageBay,
    PenderIslandOtterBay,
    SaltSpringIslandLongHarbour,
    SaturnaIslandLyallHarbour,
    VancouverTsawwassen,
    VictoriaSwartzBay,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TerminalPair {
    pub from: Terminal,
    pub to: Terminal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StopType {
    Stop,
    Transfer,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Stop {
    pub type_: StopType,
    pub terminal: Terminal,
}

#[derive(Clone, Debug)]
pub struct Sailing {
    pub depart_time: NaiveTime,
    pub arrive_time: NaiveTime,
    pub stops: Vec<Stop>,
    pub annotations: Vec<&'static str>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RouteGroup {
    SaltSpringAndOuterGulfIslands,
}

#[derive(Clone, Debug)]
pub struct ScheduleWeekday {
    pub only_dates: HashSet<NaiveDate>,
}

#[derive(Debug)]
pub struct ScheduleItem {
    pub sailing: Sailing,
    pub except_dates: HashSet<NaiveDate>,
    pub weekdays: HashMap<Weekday, ScheduleWeekday>,
}

#[derive(Debug)]
pub struct Schedule {
    pub terminal_pair: TerminalPair,
    pub effective_date_range: DateRange,
    pub items: Vec<ScheduleItem>,
    pub source_url: String,
    pub route_group: RouteGroup,
    pub reservable: bool,
}

impl Terminal {
    pub fn from_schedule_code(code: &str) -> Result<Terminal> {
        match code {
            "PLH" => Ok(Terminal::SaltSpringIslandLongHarbour),
            "POB" => Ok(Terminal::PenderIslandOtterBay),
            "PSB" => Ok(Terminal::GalianoIslandSturdiesBay),
            "PST" => Ok(Terminal::SaturnaIslandLyallHarbour),
            "PVB" => Ok(Terminal::MayneIslandVillageBay),
            "SWB" => Ok(Terminal::VictoriaSwartzBay),
            _ => Err(anyhow!("Unknown BC Ferries terminal code: {}", code)),
        }
    }

    pub fn from_schedule_stop_text(stop_text: &str) -> Result<Terminal> {
        match stop_text {
            "Mayne" => Ok(Terminal::MayneIslandVillageBay),
            "Pender" => Ok(Terminal::PenderIslandOtterBay),
            "Saturna" => Ok(Terminal::SaturnaIslandLyallHarbour),
            "Galiano" => Ok(Terminal::GalianoIslandSturdiesBay),
            "Salt Spring" => Ok(Terminal::SaltSpringIslandLongHarbour),
            _ => Err(anyhow!("Unknown stop name: {}", stop_text)),
        }
    }
}

impl fmt::Display for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Terminal::GalianoIslandSturdiesBay => write!(f, "Galiano Island (Sturdies Bay)"),
            Terminal::MayneIslandVillageBay => write!(f, "Mayne Island (Village Bay)"),
            Terminal::PenderIslandOtterBay => write!(f, "Pender Island (Otter Bay)"),
            Terminal::SaltSpringIslandLongHarbour => write!(f, "Salt Spring (Long Harbour)"),
            Terminal::SaturnaIslandLyallHarbour => write!(f, "Saturna Island (Lyall Harbour)"),
            Terminal::VancouverTsawwassen => write!(f, "Vancouver (Tsawwassen)"),
            Terminal::VictoriaSwartzBay => write!(f, "Victoria (Swartz Bay)"),
        }
    }
}

impl TerminalPair {
    pub fn from_schedule_codes(elem_id: &str) -> Result<TerminalPair> {
        let mut elem_id_parts = elem_id.split('-');
        let from = Terminal::from_schedule_code(
            elem_id_parts.next().ok_or_else(|| anyhow!("Missing first part separated by '-': {}", elem_id))?,
        )
        .context(format!("Invalid first part separated by '-': {}", elem_id))?;
        let to = Terminal::from_schedule_code(
            elem_id_parts.next().ok_or_else(|| anyhow!("Missing second part separated by '-': {}", elem_id))?,
        )
        .context(format!("Invalid second part separated by '-': {}", elem_id))?;
        ensure!(elem_id_parts.next().is_none(), "Should have only two parts separated by '-'");
        Ok(TerminalPair { from, to })
    }
}

impl fmt::Display for TerminalPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}", self.from, self.to)
    }
}

impl Stop {
    pub fn from_schedule_text(stop_text: &str) -> Result<Stop> {
        if let Some(captures) = regex!(r"^[Tt]ransfer( at)? (.*)$").captures(stop_text) {
            Ok(Stop {
                type_: StopType::Transfer,
                terminal: Terminal::from_schedule_stop_text(captures.get(2).unwrap().as_str())?,
            })
        } else {
            Ok(Stop { type_: StopType::Stop, terminal: Terminal::from_schedule_stop_text(stop_text)? })
        }
    }
}

impl DateRange {
    pub fn iter_days(&self) -> impl Iterator<Item = NaiveDate> + '_ {
        self.from.iter_days().take_while(|d| d <= &self.to)
    }

    pub fn date_within_inclusive(&self, date: NaiveDate) -> bool {
        date >= self.from && date <= self.to
    }

    pub fn from_schedule_text(range_text: &str) -> Result<DateRange> {
        const DATE_FORMAT: &str = "%B %e, %Y";
        let mut range_parts = range_text.split(" - ");
        let from = NaiveDate::parse_from_str(
            range_parts.next().ok_or_else(|| anyhow!("Missing first part separated by ' - ': {}", range_text))?,
            DATE_FORMAT,
        )
        .context(format!("Invalid first part separated by ' - ': {}", range_text))?;
        let to = NaiveDate::parse_from_str(
            range_parts.next().ok_or_else(|| anyhow!("Missing second part separated by ' - ': {}", range_text))?,
            DATE_FORMAT,
        )
        .context(format!("Invalid second part separated by ' - ': {}", range_text))?;
        ensure!(range_parts.next().is_none(), format!("Should have only two parts separated by ' - ': {}", range_text));
        Ok(DateRange { from, to })
    }
}

impl fmt::Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}", self.from, self.to)
    }
}
