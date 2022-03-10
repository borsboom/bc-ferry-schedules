use crate::annotations::*;
use crate::imports::*;

#[derive(Debug)]
pub struct WeekdayDates {
    pub map: HashMap<Weekday, AnnotationDates>,
}

impl WeekdayDates {
    pub fn new() -> WeekdayDates {
        WeekdayDates { map: HashMap::new() }
    }

    fn day_mut(&mut self, day: Weekday) -> &mut AnnotationDates {
        self.day(day);
        self.map.get_mut(&day).unwrap()
    }

    fn day(&mut self, day: Weekday) {
        self.map.entry(day).or_insert_with(AnnotationDates::new);
    }

    fn days<I: IntoIterator<Item = Weekday>>(&mut self, days: I) {
        for day in days {
            self.day(day);
        }
    }

    fn day_only(&mut self, day: Weekday, dates: &AnnotationDates) {
        self.day_mut(day).only.extend(&dates.only);
    }

    fn day_restriction(&mut self, day: Weekday, dates: &AnnotationDates) {
        self.day_mut(day).extend(dates);
    }

    fn only_date(&mut self, date: NaiveDate) {
        self.day_mut(date.weekday()).only.insert(date);
    }

    pub fn parse(orig_text: &str, annotations: &Annotations, date_range: &DateRange) -> Result<WeekdayDates> {
        let inner = || {
            let mut result = WeekdayDates::new();
            let from_year = date_range.from.year();
            let normalized_text = match orig_text {
                "Sun & Hol Mon" => "Sun, Hol Mon",
                "Fri & Apr 14 only" => "Fri, Apr 14",
                "Mon* to Sat" => "Mon*-Sat",
                "Mon*-Thu and Jan 21 & 28" => "Mon*-Thu, Jan 21, Jan 28",
                "Jan 21 & 28 only" => "Jan 21, Jan 28",
                "Dec 23 & 30 only" => "Dec 23, Dec 30",
                "Dec 26 only" => "Dec 26",
                "Fri, Hol Mon & Apr 14 only" => "Fri, Hol Mon, Apr 14",
                "Sat, Sun & Hol Mon" => "Sat, Sun, Hol Mon",
                "Dec 26 & 27 only" => "Dec 26, Dec 27",
                "Fri-Sun, Hol Mon & Apr 14 only" => "Fri-Sun, Hol Mon, Apr 14",
                text => text,
            };
            for split_text in normalized_text.split(',').map(|s| s.trim().to_lowercase()) {
                if let Ok(parsed_date) = NaiveDate::parse_from_str(&format!("{} {}", split_text, from_year), "%b %e %Y")
                {
                    result.only_date(date_range.make_year_within(parsed_date)?);
                    continue;
                }
                match &split_text[..] {
                    "mon" => result.day(Weekday::Mon),
                    "tue" => result.day(Weekday::Tue),
                    "wed" => result.day(Weekday::Wed),
                    "thu" => result.day(Weekday::Thu),
                    "fri" => result.day(Weekday::Fri),
                    "sat" => result.day(Weekday::Sat),
                    "sun" => result.day(Weekday::Sun),
                    "hol mon" => {
                        if !result.map.get(&Weekday::Mon).map(|ad| ad.is_always()).unwrap_or(false) {
                            result.day_only(Weekday::Mon, &annotations.holiday_monday)
                        }
                    }
                    "mon-thu" => result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu]),
                    "mon-fri" => result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri]),
                    "mon-sat" => result.days([
                        Weekday::Mon,
                        Weekday::Tue,
                        Weekday::Wed,
                        Weekday::Thu,
                        Weekday::Fri,
                        Weekday::Sat,
                    ]),
                    "fri-sun" => result.days([Weekday::Fri, Weekday::Sat, Weekday::Sun]),
                    "sat**" => result.day_restriction(Weekday::Sat, &annotations.starstar),
                    "sun**" => result.day_restriction(Weekday::Sun, &annotations.starstar),
                    "mon*-thu" => {
                        result.day_restriction(Weekday::Mon, &annotations.star);
                        result.days([Weekday::Tue, Weekday::Wed, Weekday::Thu]);
                    }
                    "mon-thu**" => {
                        result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed]);
                        result.day_restriction(Weekday::Thu, &annotations.starstar);
                    }
                    "mon*-thu**" => {
                        result.day_restriction(Weekday::Mon, &annotations.star);
                        result.days([Weekday::Tue, Weekday::Wed]);
                        result.day_restriction(Weekday::Thu, &annotations.starstar);
                    }
                    "mon*-fri" => {
                        result.day_restriction(Weekday::Mon, &annotations.star);
                        result.days([Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri]);
                    }
                    "mon-sat**" => {
                        result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri]);
                        result.day_restriction(Weekday::Sat, &annotations.starstar);
                    }
                    "mon*-sat" => {
                        result.day_restriction(Weekday::Mon, &annotations.star);
                        result.days([Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat]);
                    }
                    _ => bail!("Unrecognized days item text: {:?}", split_text),
                }
            }
            Ok(result)
        };
        inner().with_context(|| format!("Failed to parse schedule weekdays text: {:?}", orig_text))
    }

    pub fn to_date_restrictions(&self, row_dates: &AnnotationDates) -> HashMap<Weekday, DateRestriction> {
        let mut weekdays = HashMap::new();
        for (weekday, mut weekday_dates) in self.map.clone() {
            weekday_dates.only.extend(row_dates.only.iter().filter(|d| d.weekday() == weekday));
            weekday_dates.except.extend(row_dates.except.iter().filter(|d| d.weekday() == weekday));
            weekdays.insert(weekday, weekday_dates.into_date_restriction_by_weekday(weekday));
        }
        weekdays
    }
}
