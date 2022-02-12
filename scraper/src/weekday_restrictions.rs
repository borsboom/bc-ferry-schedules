use crate::annotations::*;
use crate::imports::*;
use crate::types::*;
use crate::utils::*;

pub struct WeekdayRestrictions {
    pub map: HashMap<Weekday, DateRestriction>,
}

impl WeekdayRestrictions {
    fn new() -> WeekdayRestrictions {
        WeekdayRestrictions { map: HashMap::new() }
    }

    fn day_mut(&mut self, day: Weekday) -> &mut DateRestriction {
        self.day(day);
        self.map.get_mut(&day).unwrap()
    }

    fn day(&mut self, day: Weekday) {
        self.map.entry(day).or_insert_with(DateRestriction::new);
    }

    fn days<I: IntoIterator<Item = Weekday>>(&mut self, days: I) {
        for day in days {
            self.day(day);
        }
    }

    fn day_only(&mut self, day: Weekday, restriction: &DateRestriction) {
        self.day_mut(day).only.extend(&restriction.only);
    }

    fn day_restriction(&mut self, day: Weekday, restriction: &DateRestriction) {
        self.day_mut(day).extend(restriction);
    }

    fn only_date(&mut self, date: NaiveDate) {
        self.day_mut(date.weekday()).only.insert(date);
    }

    pub fn parse(
        orig_text: &str,
        annotations: &Annotations,
        effective_date_range: &DateRange,
    ) -> Result<WeekdayRestrictions> {
        let mut result = WeekdayRestrictions::new();
        let from_year = effective_date_range.from.year();
        let to_year = effective_date_range.to.year();
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
        for split_text in normalized_text.split(',').map(|s| s.trim()) {
            if let Ok(parsed_date) = NaiveDate::parse_from_str(&format!("{} {}", split_text, from_year), "%b %e %Y") {
                let fixed_date = if effective_date_range.date_within_inclusive(parsed_date) {
                    parsed_date
                } else {
                    date(to_year, parsed_date.month(), parsed_date.day())
                };
                ensure!(
                    effective_date_range.date_within_inclusive(fixed_date),
                    "Date in {:?} not within effective date range: {:?}",
                    orig_text,
                    fixed_date
                );
                result.only_date(fixed_date);
                continue;
            }
            match split_text {
                "Thu" => result.day(Weekday::Thu),
                "Fri" => result.day(Weekday::Fri),
                "Sat" => result.day(Weekday::Sat),
                "Sun" => result.day(Weekday::Sun),
                "Hol Mon" => result.day_only(Weekday::Mon, &annotations.holiday_monday),
                "Mon-Thu" => result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu]),
                "Mon-Fri" => result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri]),
                "Mon-Sat" => {
                    result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat])
                }
                "Fri-Sun" => result.days([Weekday::Fri, Weekday::Sat, Weekday::Sun]),
                "Sat**" => result.day_restriction(Weekday::Sat, &annotations.starstar),
                "Sun**" => result.day_restriction(Weekday::Sun, &annotations.starstar),
                "Mon*-Thu" => {
                    result.day_restriction(Weekday::Mon, &annotations.star);
                    result.days([Weekday::Tue, Weekday::Wed, Weekday::Thu]);
                }
                "Mon-Thu**" => {
                    result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed]);
                    result.day_restriction(Weekday::Thu, &annotations.starstar);
                }
                "Mon*-Thu**" => {
                    result.day_restriction(Weekday::Mon, &annotations.star);
                    result.days([Weekday::Tue, Weekday::Wed]);
                    result.day_restriction(Weekday::Thu, &annotations.starstar);
                }
                "Mon*-Fri" => {
                    result.day_restriction(Weekday::Mon, &annotations.star);
                    result.days([Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri]);
                }
                "Mon-Sat**" => {
                    result.days([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri]);
                    result.day_restriction(Weekday::Sat, &annotations.starstar);
                }
                "Mon*-Sat" => {
                    result.day_restriction(Weekday::Mon, &annotations.star);
                    result.days([Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat]);
                }
                _ => bail!("Unrecognized days text in {:?}: {:?}", orig_text, split_text),
            }
        }
        Ok(result)
    }

    pub fn into_schedule_weekdays(
        self,
        row_date_restriction: DateRestriction,
    ) -> Result<(HashMap<Weekday, ScheduleWeekday>, HashSet<NaiveDate>)> {
        let mut weekdays = HashMap::new();
        let mut except_dates = row_date_restriction
            .except
            .into_iter()
            .filter(|d| self.map.contains_key(&d.weekday()))
            .collect::<HashSet<_>>();
        for (weekday, mut weekday_dr) in self.map {
            weekday_dr.normalize();
            weekdays.insert(weekday, ScheduleWeekday { only_dates: weekday_dr.only });
            except_dates.extend(weekday_dr.except.iter().filter(|d| d.weekday() == weekday));
        }
        for (weekday, schedule_weekday) in &mut weekdays {
            schedule_weekday.only_dates.extend(row_date_restriction.only.iter().filter(|d| d.weekday() == *weekday));
        }
        Ok((weekdays, except_dates))
    }
}
