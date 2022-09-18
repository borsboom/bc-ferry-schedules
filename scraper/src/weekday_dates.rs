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
        self.map.get_mut(&day).expect("weekday to be in map after inserting")
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

    fn only_date(&mut self, date: Date) {
        self.day_mut(date.weekday()).only.insert(date);
    }

    fn except_date(&mut self, date: Date) {
        self.day_mut(date.weekday()).except.insert(date);
    }

    pub fn parse(orig_text: &str, annotations: &Annotations, date_range: &DateRange) -> Result<WeekdayDates> {
        let inner = || {
            let mut result = WeekdayDates::new();
            let from_year = date_range.from.year();
            // TODO: generalize these rules (e.g. ` & ` and ` to ` become `, ` and `-`)
            let normalized_text = match orig_text {
                "Dec 22 & 27 only" => "Dec 22, Dec 27",
                "Dec 22, 27" => "Dec 22, Dec 27",
                "Dec 23 & 30 only" => "Dec 23, Dec 30",
                "Dec 26 & 27 only" => "Dec 26, Dec 27",
                "Dec 26 & Jan 2" => "Dec 26, Jan 2",
                "Dec 26 only" => "Dec 26",
                "Dec 26-27" => "Dec 26, Dec 27",
                "Fri & Apr 14 only" => "Fri, Apr 14",
                "Fri & Sun" => "Fri, Sun",
                "Fri-Sun & Hol Mon" => "Fri-Sun, Hol Mon",
                "Fri-Sun, Hol Mon & Apr 14 only" => "Fri-Sun, Hol Mon, Apr 14",
                "Fri, Hol Mon & Apr 14 only" => "Fri, Hol Mon, Apr 14",
                "Jan 21 & 28 only" => "Jan 21, Jan 28",
                "Mon-Fri & Hol Mon" => "Mon-Fri, Hol Mon",
                "Mon-Fri, Hol Mon except May 30" => "Mon-Fri, Hol Mon, except May 30",
                "Mon-Sat & Hol Mon" => "Mon-Sat, Hol Mon",
                "Mon-Thu & Hol Mon" => "Mon-Thu, Hol Mon",
                "Mon-Thu, Sun & Hol Mon" => "Mon-Thu, Sun, Hol Mon",
                "Mon-Thu* & Hol Mon" => "Mon-Thu*, Hol Mon",
                "Mon* to Sat" => "Mon*-Sat",
                "Mon*-Thu and Jan 21 & 28" => "Mon*-Thu, Jan 21, Jan 28",
                "Sat, Sun & Hol Mon" => "Sat, Sun, Hol Mon",
                "Sep 12, 26 & Oct 10" => "Sep 12, Sep 26, Oct 10",
                "Sep 19 & Oct 3" => "Sep 19, Oct 3",
                "Sun & Hol Mon" => "Sun, Hol Mon",
                "Sun & Oct 10 only" => "Sun, Oct 10",
                "Sat, Sun & Oct 10 only" => "Sat, Sun, Oct 10",
                text => text,
            };
            for split_text in normalized_text.split(',').map(|s| s.trim().to_lowercase()) {
                if let Ok(parsed_date) = Date::parse(
                    &format!("{} {}", split_text, from_year),
                    format_description!("[month repr:short case_sensitive:false] [day padding:none] [year]"),
                ) {
                    result.only_date(date_range.make_year_within(parsed_date)?);
                    continue;
                }
                match &split_text[..] {
                    "mon" => result.day(Weekday::Monday),
                    "tue" => result.day(Weekday::Tuesday),
                    "wed" => result.day(Weekday::Wednesday),
                    "thu" => result.day(Weekday::Thursday),
                    "fri" => result.day(Weekday::Friday),
                    "sat" => result.day(Weekday::Saturday),
                    "sun" => result.day(Weekday::Sunday),
                    "hol mon" => {
                        if !result.map.get(&Weekday::Monday).map(|ad| ad.is_always()).unwrap_or(false) {
                            result.day_only(Weekday::Monday, &annotations.holiday_monday)
                        }
                    }
                    "mon-wed" => result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday]),
                    "mon-thu" => {
                        result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday])
                    }
                    "mon-fri" => result.days([
                        Weekday::Monday,
                        Weekday::Tuesday,
                        Weekday::Wednesday,
                        Weekday::Thursday,
                        Weekday::Friday,
                    ]),
                    "mon-sat" => result.days([
                        Weekday::Monday,
                        Weekday::Tuesday,
                        Weekday::Wednesday,
                        Weekday::Thursday,
                        Weekday::Friday,
                        Weekday::Saturday,
                    ]),
                    "thu-fri" => result.days([Weekday::Thursday, Weekday::Friday]),
                    "thu-sat" => result.days([Weekday::Thursday, Weekday::Friday, Weekday::Saturday]),
                    "fri-sun" => result.days([Weekday::Friday, Weekday::Saturday, Weekday::Sunday]),
                    "sat-sun" => result.days([Weekday::Saturday, Weekday::Sunday]),
                    "thu**" => result.day_restriction(Weekday::Thursday, &annotations.star2),
                    "sat**" => result.day_restriction(Weekday::Saturday, &annotations.star2),
                    "sun**" => result.day_restriction(Weekday::Sunday, &annotations.star2),
                    "sun***" => result.day_restriction(Weekday::Sunday, &annotations.star3),
                    "mon*-thu" => {
                        result.day_restriction(Weekday::Monday, &annotations.star);
                        result.days([Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday]);
                    }
                    "mon*-wed" => {
                        result.day_restriction(Weekday::Monday, &annotations.star);
                        result.days([Weekday::Tuesday, Weekday::Wednesday]);
                    }
                    "mon-thu**" => {
                        result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday]);
                        result.day_restriction(Weekday::Thursday, &annotations.star2);
                    }
                    "mon-thu*" => {
                        result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday]);
                        result.day_restriction(Weekday::Thursday, &annotations.star);
                    }
                    "mon*-thu**" => {
                        result.day_restriction(Weekday::Monday, &annotations.star);
                        result.days([Weekday::Tuesday, Weekday::Wednesday]);
                        result.day_restriction(Weekday::Thursday, &annotations.star2);
                    }
                    "mon*-fri" => {
                        result.day_restriction(Weekday::Monday, &annotations.star);
                        result.days([Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday]);
                    }
                    "mon-sat**" => {
                        result.days([
                            Weekday::Monday,
                            Weekday::Tuesday,
                            Weekday::Wednesday,
                            Weekday::Thursday,
                            Weekday::Friday,
                        ]);
                        result.day_restriction(Weekday::Saturday, &annotations.star2);
                    }
                    "mon*-sat" => {
                        result.day_restriction(Weekday::Monday, &annotations.star);
                        result.days([
                            Weekday::Tuesday,
                            Weekday::Wednesday,
                            Weekday::Thursday,
                            Weekday::Friday,
                            Weekday::Saturday,
                        ]);
                    }
                    "except may 30" => {
                        result.except_date(date_range.make_year_within(Date::from_calendar_date(
                            from_year,
                            Month::May,
                            30,
                        )?)?);
                    }
                    "mon-thu except oct 10" => {
                        result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday]);
                        result.day_restriction(
                            Weekday::Monday,
                            &AnnotationDates::except(&[date_range.make_year_within(Date::from_calendar_date(
                                from_year,
                                Month::October,
                                10,
                            )?)?]),
                        );
                    }
                    "mon-fri except oct 10" => {
                        result.days([
                            Weekday::Monday,
                            Weekday::Tuesday,
                            Weekday::Wednesday,
                            Weekday::Thursday,
                            Weekday::Friday,
                        ]);
                        result.day_restriction(
                            Weekday::Monday,
                            &AnnotationDates::except(&[date_range.make_year_within(Date::from_calendar_date(
                                from_year,
                                Month::October,
                                10,
                            )?)?]),
                        );
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
