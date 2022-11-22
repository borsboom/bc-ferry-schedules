use crate::annotations::*;
use crate::imports::*;

#[derive(Debug)]
pub struct WeekdayDates {
    pub map: HashMap<Weekday, AnnotationDates>,
    pub notes: AnnotationNotes,
}

impl WeekdayDates {
    pub fn new() -> WeekdayDates {
        WeekdayDates { map: HashMap::new(), notes: AnnotationNotes::new() }
    }

    fn day_mut(&mut self, day: Weekday) -> &mut AnnotationDates {
        self.day(day);
        self.map.get_mut(&day).expect("Expect weekday to be in map after inserting")
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

    fn note(&mut self, note: Cow<'static, str>) {
        self.notes.map.insert(note, AnnotationDates::new());
    }

    pub fn parse(orig_text: &str, annotations: &Annotations, date_range: &DateRange) -> Result<WeekdayDates> {
        let inner = || {
            let mut result = WeekdayDates::new();
            let from_year = date_range.from.year();
            // TODO: generalize these rules (e.g. ` & ` and ` to ` become `, ` and `-`; remove ` only` suffix)
            let normalized_text = match orig_text {
                "Apr 10 only" => "Apr 10",
                "Apr 6 only" => "Apr 6",
                "Dec 22 & 27 only" => "Dec 22, Dec 27",
                "Dec 22, 27" => "Dec 22, Dec 27",
                "Dec 23 & 30 only" => "Dec 23, Dec 30",
                "Dec 23 only" => "Jan 23",
                "Dec 26 & 27 only" => "Dec 26, Dec 27",
                "Dec 26 & Jan 2" => "Dec 26, Jan 2",
                "Dec 26 only" => "Dec 26",
                "Dec 26-27 only" => "Dec 26, Dec 27",
                "Dec 26-27" => "Dec 26, Dec 27",
                "Fri & Apr 14 only" => "Fri, Apr 14",
                "Fri & Apr 6 only" => "Fri, Apr 6",
                "Fri & Sun" => "Fri, Sun",
                "Fri-Sun & Hol Mon" => "Fri-Sun, Hol Mon",
                "Fri-Sun, Hol Mon & Apr 14 only" => "Fri-Sun, Hol Mon, Apr 14",
                "Fri, Hol Mon & Apr 14 only" => "Fri, Hol Mon, Apr 14",
                "Jan 21 & 28 only" => "Jan 21, Jan 28",
                "Mar 28 only" => "Mar 28",
                "Mon-Fri & Hol Mon" => "Mon-Fri, Hol Mon",
                "Mon-Fri, Hol Mon except May 30" => "Mon-Fri, Hol Mon, except May 30",
                "Mon-Sat & Hol Mon" => "Mon-Sat, Hol Mon",
                "Mon-Thu & Hol Mon" => "Mon-Thu, Hol Mon",
                "Mon-Thu, Sun & Hol Mon" => "Mon-Thu, Sun, Hol Mon",
                "Mon-Thu* & Hol Mon" => "Mon-Thu*, Hol Mon",
                "Mon* to Sat" => "Mon*-Sat",
                "Mon*-Thu and Jan 21 & 28" => "Mon*-Thu, Jan 21, Jan 28",
                "Nov 13, Feb 19 & Mar 28 only" => "Nov 13, Feb 19, Mar 28",
                "Sat-Sun & Apr 10 only" => "Sat-Sun, Apr 10",
                "Sat-Sun & May 22 only" => "Sat-Sun, May 22",
                "Sat-Sun & Nov 13, Feb 19 & Mar 28 only" => "Sat-Sun, Nov 13, Feb 19, Mar 28",
                "Sat-Sun & Oct 9 only" => "Sat-Sun, Oct 9",
                "Sat, Sun & Hol Mon" => "Sat, Sun, Hol Mon",
                "Sat, Sun & Oct 10 only" => "Sat, Sun, Oct 10",
                "Sep 12, 26 & Oct 10" => "Sep 12, Sep 26, Oct 10",
                "Sep 19 & Oct 3" => "Sep 19, Oct 3",
                "Sun & Apr 10 only" => "Sun, Apr 10",
                "Sun & Aug 7 & Sep 4 only" | "Sun and Aug 7 & Sep 4 only" => "Sun, Aug 7, Sep 4",
                "Sun & Hol Mon" => "Sun, Hol Mon",
                "Sun & May 22 only" => "Sun, May 22",
                "Sun & Nov 13, Feb 19 & Mar 28 only" | "Sun and Nov 13, Feb 19 & Mar 28 only" => {
                    "Sun, Nov 13, Feb 19, Mar 28"
                }
                "Sun & Oct 10 only" => "Sun, Oct 10",
                "Sun & Oct 9 only" => "Sun, Oct 9",
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
                    "mon" | "mondays" => result.day(Weekday::Monday),
                    "tue" | "tuesdays" => result.day(Weekday::Tuesday),
                    "wed" | "wednesdays" => result.day(Weekday::Wednesday),
                    "thu" | "thursdays" => result.day(Weekday::Thursday),
                    "fri" | "fridays" => result.day(Weekday::Friday),
                    "sat" | "saturdays" => result.day(Weekday::Saturday),
                    "sun" | "sundays" => result.day(Weekday::Sunday),
                    "hol mon" => {
                        if !result.map.get(&Weekday::Monday).map(|ad| ad.is_always()).unwrap_or(false) {
                            result.day_only(Weekday::Monday, &annotations.holiday_monday_dates)
                        }
                    }
                    "sun-thu" => result.days([
                        Weekday::Sunday,
                        Weekday::Monday,
                        Weekday::Tuesday,
                        Weekday::Wednesday,
                        Weekday::Thursday,
                    ]),
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
                    "tue-thu" => result.days([Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday]),
                    "tue-fri" => {
                        result.days([Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday])
                    }
                    "thu-fri" => result.days([Weekday::Thursday, Weekday::Friday]),
                    "thu-sat" => result.days([Weekday::Thursday, Weekday::Friday, Weekday::Saturday]),
                    "fri-sun" => result.days([Weekday::Friday, Weekday::Saturday, Weekday::Sunday]),
                    "sat-sun" => result.days([Weekday::Saturday, Weekday::Sunday]),
                    "thu**" => result.day_restriction(Weekday::Thursday, &annotations.star2_dates),
                    "sat**" => result.day_restriction(Weekday::Saturday, &annotations.star2_dates),
                    "sun**" => result.day_restriction(Weekday::Sunday, &annotations.star2_dates),
                    "sun***" => result.day_restriction(Weekday::Sunday, &annotations.star3_dates),
                    "dg sun" => {
                        result.day_restriction(Weekday::Sunday, &annotations.dangerous_goods_dates);
                        result.note(Cow::from("Dangerous goods sailing only, no other passengers permitted"));
                    }
                    "mon*" => {
                        result.day_restriction(Weekday::Monday, &annotations.star_dates);
                    }
                    "mon*-thu" => {
                        result.day_restriction(Weekday::Monday, &annotations.star_dates);
                        result.days([Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday]);
                    }
                    "mon*-wed" => {
                        result.day_restriction(Weekday::Monday, &annotations.star_dates);
                        result.days([Weekday::Tuesday, Weekday::Wednesday]);
                    }
                    "mon-thu**" => {
                        result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday]);
                        result.day_restriction(Weekday::Thursday, &annotations.star2_dates);
                    }
                    "mon-thu*" => {
                        result.days([Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday]);
                        result.day_restriction(Weekday::Thursday, &annotations.star_dates);
                    }
                    "mon*-thu**" => {
                        result.day_restriction(Weekday::Monday, &annotations.star_dates);
                        result.days([Weekday::Tuesday, Weekday::Wednesday]);
                        result.day_restriction(Weekday::Thursday, &annotations.star2_dates);
                    }
                    "mon*-fri" => {
                        result.day_restriction(Weekday::Monday, &annotations.star_dates);
                        result.days([Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday]);
                    }
                    "mon**-fri" => {
                        result.day_restriction(Weekday::Monday, &annotations.star2_dates);
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
                        result.day_restriction(Weekday::Saturday, &annotations.star2_dates);
                    }
                    "mon*-sat" => {
                        result.day_restriction(Weekday::Monday, &annotations.star_dates);
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
