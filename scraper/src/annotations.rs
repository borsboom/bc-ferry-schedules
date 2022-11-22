use crate::constants::*;
use crate::imports::*;
use crate::macros::*;

#[derive(Clone, Debug)]
pub struct AnnotationDates {
    pub only: HashSet<Date>,
    pub except: HashSet<Date>,
}

#[derive(Clone, Debug)]
pub struct AnnotationNotes {
    pub map: HashMap<Cow<'static, str>, AnnotationDates>,
}

#[derive(Debug)]
pub struct Annotations {
    // TODO: reduce repetition
    pub holiday_monday_dates: AnnotationDates,
    pub dangerous_goods_dates: AnnotationDates,
    pub star_dates: AnnotationDates,
    pub star_dates_by_time: HashMap<Time, AnnotationDates>,
    pub star2_dates: AnnotationDates,
    pub star3_dates: AnnotationDates,
    pub exclamation_dates: AnnotationDates,
    pub exclamation_notes: AnnotationNotes,
    pub exclamation2_notes: AnnotationNotes,
    pub hash_dates: AnnotationDates,
    pub hash_notes: AnnotationNotes,
    pub plus_dates: AnnotationDates,
    pub plus_notes: AnnotationNotes,
    pub all_dates: AnnotationDates,
}

fn text_date_restriction<T: Into<Cow<'static, str>>>(notes: &mut AnnotationNotes, text: T) -> &mut AnnotationDates {
    notes.map.entry(text.into()).or_insert_with(AnnotationDates::new)
}

pub fn annotation_notes_date_restictions(
    row_notes: AnnotationNotes,
    weekday_notes: AnnotationNotes,
    weekdays: &HashMap<Weekday, DateRestriction>,
) -> HashMap<Cow<'static, str>, DateRestriction> {
    AnnotationDates::map_to_date_restrictions_by_weekdays(
        row_notes.map.into_iter().chain(weekday_notes.map.into_iter()),
        weekdays,
    )
}

impl AnnotationDates {
    pub fn new() -> AnnotationDates {
        AnnotationDates { only: HashSet::new(), except: HashSet::new() }
    }

    pub fn only<'a, I: IntoIterator<Item = &'a Date>>(only: I) -> AnnotationDates {
        AnnotationDates { except: HashSet::new(), only: only.into_iter().cloned().collect() }
    }

    pub fn except<'a, I: IntoIterator<Item = &'a Date>>(except: I) -> AnnotationDates {
        AnnotationDates { only: HashSet::new(), except: except.into_iter().cloned().collect() }
    }

    pub fn is_always(&self) -> bool {
        self.only.is_empty() && self.except.is_empty()
    }

    pub fn extend(&mut self, other: &AnnotationDates) {
        self.except.extend(&other.except);
        self.only.extend(&other.only);
    }

    fn into_date_restriction(mut self) -> DateRestriction {
        let common_dates: Vec<_> = self.except.intersection(&self.only).copied().collect();
        for common_date in common_dates {
            self.except.remove(&common_date);
            self.only.remove(&common_date);
        }
        if !self.only.is_empty() {
            DateRestriction::Only(self.only)
        } else if !self.except.is_empty() {
            DateRestriction::Except(self.except)
        } else {
            DateRestriction::All
        }
    }

    fn into_date_restriction_by<F>(mut self, predicate: F) -> DateRestriction
    where
        F: Fn(&Date) -> bool,
    {
        self.only.retain(&predicate);
        self.except.retain(&predicate);
        self.into_date_restriction()
    }

    pub fn into_date_restriction_by_weekday(self, weekday: Weekday) -> DateRestriction {
        self.into_date_restriction_by(|d| d.weekday() == weekday)
    }

    pub fn into_date_restriction_by_weekdays(self, weekdays: &HashMap<Weekday, DateRestriction>) -> DateRestriction {
        self.into_date_restriction_by(|date: &Date| {
            weekdays.get(&date.weekday()).map(|dr| dr.includes_date(*date)).unwrap_or(false)
        })
    }

    pub fn map_to_date_restrictions_by_weekdays<I, K>(
        map: I,
        weekdays: &HashMap<Weekday, DateRestriction>,
    ) -> HashMap<K, DateRestriction>
    where
        K: Eq + Hash,
        I: IntoIterator<Item = (K, AnnotationDates)>,
    {
        map.into_iter()
            .filter_map(|(k, ad)| {
                let dr = ad.into_date_restriction_by_weekdays(weekdays);
                (!dr.is_never()).then(|| (k, dr))
            })
            .collect()
    }
}

impl AnnotationNotes {
    pub fn new() -> AnnotationNotes {
        AnnotationNotes { map: HashMap::new() }
    }

    pub fn extend(&mut self, other: AnnotationNotes) {
        self.map.extend(other.map.into_iter());
    }
}

impl Annotations {
    pub fn new(date_range: &DateRange) -> Annotations {
        Annotations {
            holiday_monday_dates: AnnotationDates::only(
                EXTRA_HOLIDAY_MONDAYS.iter().filter(|d| date_range.includes_date_inclusive(**d)),
            ),
            dangerous_goods_dates: AnnotationDates::new(),
            star_dates: AnnotationDates::new(),
            star_dates_by_time: HashMap::new(),
            star2_dates: AnnotationDates::new(),
            star3_dates: AnnotationDates::new(),
            exclamation_dates: AnnotationDates::new(),
            exclamation_notes: AnnotationNotes::new(),
            exclamation2_notes: AnnotationNotes::new(),
            hash_dates: AnnotationDates::new(),
            hash_notes: AnnotationNotes::new(),
            plus_dates: AnnotationDates::new(),
            plus_notes: AnnotationNotes::new(),
            all_dates: AnnotationDates::new(),
        }
    }

    fn star_holiday_monday_extend(&mut self, dates: &[Date]) {
        self.holiday_monday_dates.only.extend(dates);
        self.star_dates.except.extend(dates);
    }

    pub fn parse<T: AsRef<str>, I: IntoIterator<Item = T>>(
        &mut self,
        annotation_texts: I,
        date_range: &DateRange,
    ) -> Result<()> {
        let from_year = date_range.from.year();
        let to_year = date_range.to.year();
        let schedule_year_date = |m, d| {
            let inner = || date_range.make_year_within(Date::from_calendar_date(from_year, m, d)?);
            inner().context("Invalid date for schedule in annotation")
        };
        let mut annotation_is_exclamation = false;
        for annotation_text in annotation_texts {
            let mut inner = || {
                let annotation_text = match annotation_text.as_ref() {
                    "Only on Dec 23 & 30" => "Only on Dec 23, Dec 30",
                    text => text,
                };
                let mut next_annotation_is_exclamation = false;
                if let Some(captures) =
                    regex!(r"(?i)^\*(\d+:\d+ [AP]M) (Not Available|Only) on: (.*)\*").captures(annotation_text.as_ref())
                {
                    let time_text = &captures[1];
                    let time = Time::parse(
                        time_text,
                        format_description!(
                            "[hour repr:12 padding:none]:[minute] [period case:lower case_sensitive:false]"
                        ),
                    )
                    .with_context(|| format!("Failed to parse time: {:?}", time_text))?;
                    let dates = self.star_dates_by_time.entry(time).or_insert_with(AnnotationDates::new);
                    let dates_hashset = match &captures[2] {
                        "Not Available" => &mut dates.except,
                        "Only" => &mut dates.only,
                        other => bail!("Expect \"Not Available\" or \"Only\" in: {:?}", other),
                    };
                    for date_text in captures[3].split(',').map(|s| s.trim()) {
                        let parsed_date = Date::parse(
                            &format!("{} {}", date_text, from_year),
                            format_description!("[day padding:none] [month repr:short case_sensitive:false] [year]"),
                        )
                        .with_context(|| format!("Failed to parse date: {:?}", date_text))?;
                        let date = date_range.make_year_within(parsed_date).with_context(|| {
                            format!("Date is outside date range of schedule ({}): {:?}", date_range, parsed_date)
                        })?;
                        dates_hashset.insert(date);
                    }
                } else if let Some(captures) = regex!(r"(?i)^(Except|Only) on (.*)").captures(annotation_text.as_ref())
                {
                    let dates_hashset = match &captures[1] {
                        "Except" => &mut self.all_dates.except,
                        "Only" => &mut self.all_dates.only,
                        other => bail!("Expect \"Except\" or \"Only\" in: {:?}", other),
                    };
                    for date_text in captures[2].split(&[',', '&']).map(|s| s.trim()) {
                        let parsed_date = Date::parse(
                            &format!("{} {}", date_text, from_year),
                            format_description!("[month repr:short case_sensitive:false] [day padding:none] [year]"),
                        )
                        .with_context(|| format!("Failed to parse date: {:?}", date_text))?;
                        let date = date_range.make_year_within(parsed_date).with_context(|| {
                            format!("Date is outside date range of schedule ({}): {:?}", date_range, parsed_date)
                        })?;
                        dates_hashset.insert(date);
                    }
                } else {
                    let replaced_annotation_text = regex!(r"([!#*]*)\s*").replace(annotation_text.as_ref(), "$1 ");
                    let replaced_annotation_text = regex!(r"[\.,]$").replace(replaced_annotation_text.as_ref(), "");
                    let annotation_text = replaced_annotation_text.as_ref().trim();
                    // TODO: reduce repetition
                    match annotation_text {
                        "!" => next_annotation_is_exclamation = true,
                        "No sailings available on this route for these dates" => {}
                        "* On April 18, 2022 the Holiday Monday Schedule is in effect" |
                        "* On April 18, 2022, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[
                                date!(2022 - 4 - 18),
                            ]),
                        "* On April 18th, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[
                                schedule_year_date(Month::April, 18)?,
                            ]),
                        "* On May 23rd, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On May 23, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On May 23rd the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm." =>
                            self.star_holiday_monday_extend(&[
                                schedule_year_date(Month::May, 23)?,
                            ]),
                        "* Except On December 26, 2022, January 2 & February 20, 2023" =>
                            self.star_dates.except.extend([
                                date!(2022 - 12 - 26),
                                date!(2023 - 1 - 2),
                                date!(2023 - 2 - 20),
                            ]),
                        "* Except on August 1st and September 5th 2022" =>
                            self.star_dates.except.extend([
                                date!(2022 - 8 - 1),
                                date!(2022 - 9 - 5),
                            ]),
                        "* Except Aug 1 & Sep 5" =>
                            self.star_dates.except.extend([
                                schedule_year_date(Month::August, 1)?,
                                schedule_year_date(Month::September, 5)?,
                            ]),
                        "* Except on Aug 7 & Sep 4" =>
                            self.star_dates.except.extend([
                                schedule_year_date(Month::August, 7)?,
                                schedule_year_date(Month::September, 4)?,
                            ]),
                        "* Except on Oct 9" =>
                            self.star_dates.except.extend([
                                schedule_year_date(Month::October, 9)?,
                            ]),
                        "* Except on October 10, 2022" =>
                            self.star_dates.except.extend([
                                date!(2022 - 10 - 10),
                            ]),
                        "* Except on Apr 10" =>
                            self.star_dates.except.extend([schedule_year_date(Month::April, 10)?]),
                        "* Except on April 14th" =>
                            self.star_dates.except.extend([schedule_year_date(Month::April, 14)?]),
                        "* Except on Nov 13, Feb 19 & Mar 28" =>
                            self.star_dates.except.extend([
                                schedule_year_date(Month::November, 13)?,
                                schedule_year_date(Month::February, 19)?,
                                schedule_year_date(Month::March, 28)?,
                            ]),
                        "** Except on April 14th" =>
                            self.star2_dates.except.extend([schedule_year_date(Month::April, 14)?]),
                        "* Except on April 18th" =>
                            self.star_dates.except.extend([schedule_year_date(Month::April, 18)?]),
                        "* Except on May 22" =>
                            self.star_dates.except.extend([schedule_year_date(Month::May, 22)?]),
                        "** Except on Apr 6 & 10" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::April, 6)?,
                                schedule_year_date(Month::April, 10)?,
                            ]),
                        "* Except on Apr 6 & 10" =>
                            self.star_dates.except.extend([
                                schedule_year_date(Month::April, 6)?,
                                schedule_year_date(Month::April, 10)?,
                            ]),
                        "** Except on July 3, 17, 31 August 14, 28" |
                        "** Except on Jul 3, 17, 31, Aug 14 & 28" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::July, 3)?,
                                schedule_year_date(Month::July, 17)?,
                                schedule_year_date(Month::July, 31)?,
                                schedule_year_date(Month::August, 14)?,
                                schedule_year_date(Month::August, 28)?
                            ]),
                        "** Except on July 10, 24, August 7, 21, September 4" |
                        "** Except on Jul 10, 24, Aug 7, 21 & Sep 4" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::July, 10)?,
                                schedule_year_date(Month::July, 24)?,
                                schedule_year_date(Month::August, 7)?,
                                schedule_year_date(Month::August, 21)?,
                                schedule_year_date(Month::September, 4)?
                            ]),
                            "** Except Sep 11, 25 & Oct 9" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::September, 11)?,
                                schedule_year_date(Month::September, 25)?,
                                schedule_year_date(Month::October, 9)?
                            ]),
                        "** Except on Dec 25 & Jan 1" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::December, 25)?,
                                schedule_year_date(Month::January, 1)?,
                            ]),
                        "** Except on September 18 and October 2" |
                        "** Except Sep 18 & Oct 2" |
                        "** Except on Sep 18 & Oct 2" =>
                            self.star2_dates.except.extend([schedule_year_date(Month::September, 18)?, schedule_year_date(Month::October, 2)?]),
                        "*** Except Sep 18 & Oct 2" =>
                            self.star3_dates.except.extend([schedule_year_date(Month::September, 18)?, schedule_year_date(Month::October, 2)?]),
                        "** Only on April 14th" =>
                            self.star2_dates.only.extend([schedule_year_date(Month::April, 14)?]),
                        "** Only on December 23 and December 30" |
                        "** Only on Dec 23 & 30" =>
                            self.star2_dates.only.extend([schedule_year_date(Month::December, 23)?, schedule_year_date(Month::December, 30)?]),
                        "** Only on July 3, 17, 31 August 14, 28" |
                        "** Only on July 3, 17, 31, August 14, 28" |
                        "** Only on Jul 3, 17, 31 Aug 14 & 28" |
                        "** Only on Jul 3, 17, 31, Aug 14 & 28" =>
                            self.star2_dates.only.extend([
                                schedule_year_date(Month::July, 3)?,
                                schedule_year_date(Month::July, 17)?,
                                schedule_year_date(Month::July, 31)?,
                                schedule_year_date(Month::August, 14)?,
                                schedule_year_date(Month::August, 28)?
                            ]),
                        "** Only on September 18 and October 2" |
                        "** Only on Sep 18 & Oct 2" =>
                            self.star2_dates.only.extend([schedule_year_date(Month::September, 18)?, schedule_year_date(Month::October, 2)?]),
                        "** Only on September 11, 25, October 9" =>
                            self.star2_dates.only.extend([
                                schedule_year_date(Month::September, 11)?,
                                schedule_year_date(Month::September, 25)?,
                                schedule_year_date(Month::October, 9)?
                            ]),
                        "*** Only on July 10, 24, August 7, 21, September 4" |
                        "*** Only on Jul 10, 24, Aug 7, 21 & Sep 4" =>
                            self.star3_dates.only.extend([
                                schedule_year_date(Month::July, 10)?,
                                schedule_year_date(Month::July, 24)?,
                                schedule_year_date(Month::August, 7)?,
                                schedule_year_date(Month::August, 21)?,
                                schedule_year_date(Month::September, 4)?
                            ]),
                        "*** Only on September 11, 25, October 9" |
                        "*** Only on Sep 11, 25 & Oct 9" =>
                            self.star3_dates.only.extend([
                                schedule_year_date(Month::September, 11)?,
                                schedule_year_date(Month::September, 25)?,
                                schedule_year_date(Month::October, 9)?
                            ]),
                        "* Only on April 14th" =>
                            self.star_dates.only.extend([schedule_year_date(Month::April, 14)?]),
                        "* Only on Apr 10" =>
                            self.star_dates.only.extend([schedule_year_date(Month::April, 10)?]),
                        "On April 18th the Holiday Monday schedule is in effect" if annotation_is_exclamation =>
                            self.star_holiday_monday_extend(&[schedule_year_date(Month::April, 18)?]),
                        "* On April 18th the Holiday Monday schedule is in effect" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(Month::April, 18)?]),
                        "* On May 23rd the Holiday Monday schedule is in effect" |
                        "* On May 23 the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(Month::May, 23)?]),
                        "* On August 1st and September 5th 2022, the Holiday Monday schedule is in effect" |
                        "* On August 1st and September 5th 2022, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[date!(2022 - 8 - 1), date!(2022 - 9 - 5)]),
                        "* On Aug 1 & Sep 5, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(Month::August, 1)?, schedule_year_date(Month::September, 5)?]),
                        "* On October 10, 2022, the Holiday Monday Schedule is in effect" |
                        "* On October 10, 2022, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On Oct 10th the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On Oct 10, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(Month::October, 10)?]),
                        "* On December 26, 2022, January 2 & February 20, 2023 the Holiday Monday schedule is in effect" |
                        "* On December 26, 2022, January 2 & February 20, 2023, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On Dec 26, 2022, Jan 2 & Feb 20, 2023, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[date!(2022 - 12 - 26), date!(2023 - 1 - 2), date!(2023 - 2 - 20)]),
                        "** Except February 14 to March 28, 2022" =>
                            self.star2_dates.except.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 28),
                                }.iter_days()),
                        "! Except February 14 to March 28, 2022" =>
                            self.exclamation_dates.except.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 28),
                                }.iter_days()),
                        "! Except February 14 to March 16, 2022" =>
                            self.exclamation_dates.except.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 16),
                                }.iter_days()),
                        "# Except February 14-March 28, 2022" =>
                            self.hash_dates.except.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 28),
                                }.iter_days()),
                        "# Except February 14-March 16, 2022" =>
                            self.hash_dates.except.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 16),
                                }.iter_days()),
                        "** February 14-March 28, 2022 only" =>
                            self.star2_dates.only.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 28),
                                }.iter_days()),
                        "** February 14-March 16, 2022 only" =>
                            self.star2_dates.only.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 16),
                                }.iter_days()),
                        "# February 14 to March 28, 2022 only" =>
                            self.hash_dates.only.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 28),
                                }.iter_days()),
                        "# February 14 to March 16, 2022 only" =>
                            self.hash_dates.only.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 16),
                                }.iter_days()),
                        "+ February 14 to March 28, 2022 only" =>
                            self.plus_dates.only.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 28),
                                }.iter_days()),
                        "+ February 14 to March 16, 2022 only" =>
                            self.plus_dates.only.extend(
                                DateRange {
                                    from: date!(2022 - 2 - 14),
                                    to: date!(2022 - 3 - 16),
                                }.iter_days()),
                        "# Only on February 18, 25, March 4, 11, 18, 25, 2022" |
                        "# Only on February 18, 25, March 4, 11, 18 and 25, 2022" | "# February 18, 25, March 4, 11, 18 and 25, 2022 only" =>
                            self.hash_dates.only.extend([
                                date!(2022 - 2 - 18),
                                date!(2022 - 2 - 25),
                                date!(2022 - 3 - 4),
                                date!(2022 - 3 - 11),
                                date!(2022 - 3 - 18),
                                date!(2022 - 3 - 25),
                            ]),
                        "# Only on February 18, 25, March 4, 11, 2022" |
                        "# February 18, 25, March 4, 11, 2022 only" =>
                            self.hash_dates.only.extend([
                                date!(2022 - 2 - 18),
                                date!(2022 - 2 - 25),
                                date!(2022 - 3 - 4),
                                date!(2022 - 3 - 11),
                            ]),
                        "** Except February 18, 25, March 4, 11, 18 and 25, 2022" |
                        "** Except on February 18, 25, March 4, 11, 18 and 25, 2022" =>
                            self.star2_dates.except.extend([
                                date!(2022 - 2 - 18),
                                date!(2022 - 2 - 25),
                                date!(2022 - 3 - 4),
                                date!(2022 - 3 - 11),
                                date!(2022 - 3 - 18),
                                date!(2022 - 3 - 25),
                            ]),
                        "** Except February 18, 25, March 4, 11, 2022" |
                        "** Except on February 18, 25, March 4, 11, 2022" =>
                            self.star2_dates.except.extend([
                                date!(2022 - 2 - 18),
                                date!(2022 - 2 - 25),
                                date!(2022 - 3 - 4),
                                date!(2022 - 3 - 11),
                            ]),
                        "** Except February 14 to March 16, 2022" =>
                            self.star2_dates.except.extend(DateRange {
                                from: date!(2022 - 2 - 14),
                                to: date!(2022 - 3 - 16)
                            }.iter_days()),
                        "** Except May 8, 22 & June 5, 19" |
                        "** Except May 8, 22, Jun 5 & 19" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::May, 8)?,
                                schedule_year_date(Month::May, 22)?,
                                schedule_year_date(Month::June, 5)?,
                                schedule_year_date(Month::June, 19)?,
                            ]),
                        "** Except May 8, 29 & June 5, 19" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::May, 8)?,
                                schedule_year_date(Month::May, 29)?,
                                schedule_year_date(Month::June, 5)?,
                                schedule_year_date(Month::June, 19)?,
                            ]),
                        "** Except on May 15, 29 & June 12, 26" |
                        "** Except May 15, 29, Jun 12 & 26" |
                        "** Except on May 15, 29, Jun 12 & 26" =>
                            self.star2_dates.except.extend([
                                schedule_year_date(Month::May, 15)?,
                                schedule_year_date(Month::May, 29)?,
                                schedule_year_date(Month::June, 12)?,
                                schedule_year_date(Month::June, 26)?,
                            ]),
                        "*** Except May 15, 29, Jun 12 & 26" =>
                            self.star3_dates.except.extend([
                                schedule_year_date(Month::May, 15)?,
                                schedule_year_date(Month::May, 29)?,
                                schedule_year_date(Month::June, 12)?,
                                schedule_year_date(Month::June, 26)?,
                            ]),
                        "*** Only on May 8, 22, Jun 12 & 26" =>
                            self.star3_dates.except.extend([
                                schedule_year_date(Month::May, 8)?,
                                schedule_year_date(Month::May, 22)?,
                                schedule_year_date(Month::June, 12)?,
                                schedule_year_date(Month::June, 26)?,
                            ]),
                        "# Foot passengers only on this sailing - Vehicles permitted February 14 to March 28, 2022" =>
                            text_date_restriction(&mut self.hash_notes, "Foot passengers only").except.extend(DateRange {
                                from: schedule_year_date(Month::February, 14)?,
                                to: schedule_year_date(Month::March, 28)?,
                            }.iter_days()),
                        "# Foot passengers only on this sailing - Vehicles permitted February 14 to March 16, 2022" =>
                            text_date_restriction(&mut self.hash_notes, "Foot passengers only").except.extend(DateRange {
                                from: schedule_year_date(Month::February, 14)?,
                                to: schedule_year_date(Month::March, 16)?,
                            }.iter_days()),
                        "# Foot passengers only February 14 to March 28" =>
                            text_date_restriction(&mut self.hash_notes, "Foot passengers only").only.extend(DateRange {
                                    from: schedule_year_date(Month::February, 14)?,
                                    to: schedule_year_date(Month::March, 28)?,
                                }.iter_days()),
                        "# Foot passengers only February 14 to March 16" =>
                            text_date_restriction(&mut self.hash_notes, "Foot passengers only").only.extend(DateRange {
                                    from: schedule_year_date(Month::February, 14)?,
                                    to: schedule_year_date(Month::March, 16)?,
                                }.iter_days()),
                        "+ Foot passengers only through March 28" if to_year == 2022 =>
                            text_date_restriction(&mut self.plus_notes, "Foot passengers only").only.extend(DateRange {
                                from: date!(2022 - 2 - 14),
                                to: date!(2022 - 3 - 28),
                            }.iter_days()),
                        "+ Foot passengers only through March 16" if to_year == 2022 =>
                            text_date_restriction(&mut self.plus_notes, "Foot passengers only").only.extend(DateRange {
                                from: date!(2022 - 2 - 14),
                                to: date!(2022 - 3 - 16),
                            }.iter_days()),
                        "# Foot passengers only on this sailing" => {
                            text_date_restriction(&mut self.hash_notes, "Foot passengers only");
                        }
                        "# Foot passengers only on this sailing except the 9:10 AM sailing on May 30 which will permit vehicles" => {
                            text_date_restriction(
                                &mut self.hash_notes,
                                "Foot passengers only on this sailing except the 9:10 AM sailing on May 30 which will permit vehicles"
                            );
                        }
                        "+ Foot passengers only Fridays February 18, 25, March 4, 11, 18, 25" =>
                            text_date_restriction(&mut self.plus_notes, "Foot passengers only").only.extend([
                                schedule_year_date(Month::February, 18)?,
                                schedule_year_date(Month::February, 25)?,
                                schedule_year_date(Month::March, 4)?,
                                schedule_year_date(Month::March, 11)?,
                                schedule_year_date(Month::March, 18)?,
                                schedule_year_date(Month::March, 25)?,
                            ]),
                        "! Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing" => {
                            text_date_restriction(
                                &mut self.exclamation_notes,
                                "Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing"
                            );
                        }
                        "!! On February 18, 25, March 4, 11, 18, 25 arrival time will be 5:35 PM" => {
                            text_date_restriction(&mut self.exclamation2_notes, "Arrival time will be 5:35 PM").only.extend([
                                schedule_year_date(Month::February, 18)?,
                                schedule_year_date(Month::February, 25)?,
                                schedule_year_date(Month::March, 4)?,
                                schedule_year_date(Month::March, 11)?,
                                schedule_year_date(Month::March, 18)?,
                                schedule_year_date(Month::March, 25)?,
                            ])
                        }
                        "!! On February 18, 25, March 4, 11 arrival time will be 5:35 PM" => {
                            text_date_restriction(&mut self.exclamation2_notes, "Arrival time will be 5:35 PM").only.extend([
                                schedule_year_date(Month::February, 18)?,
                                schedule_year_date(Month::February, 25)?,
                                schedule_year_date(Month::March, 4)?,
                                schedule_year_date(Month::March, 11)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Apr 9 & 23, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::April, 9)?,
                                schedule_year_date(Month::April, 23)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on May 7, 21, Jun 4 & 18, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::May, 7)?,
                                schedule_year_date(Month::May, 21)?,
                                schedule_year_date(Month::June, 4)?,
                                schedule_year_date(Month::June, 18)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Jul 2, 16, 30, Aug 13 & 27, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::July, 2)?,
                                schedule_year_date(Month::July, 16)?,
                                schedule_year_date(Month::July, 30)?,
                                schedule_year_date(Month::August, 13)?,
                                schedule_year_date(Month::August, 27)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Sep 10, 24 & Oct 9, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::September, 10)?,
                                schedule_year_date(Month::September, 24)?,
                                schedule_year_date(Month::October, 9)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Oct 22, Nov 6, 19, Dec 3, 17, 31, Jan 14, 28, Feb 11, 25, Mar 10 & 24, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::October, 22)?,
                                schedule_year_date(Month::November, 6)?,
                                schedule_year_date(Month::November, 19)?,
                                schedule_year_date(Month::December, 3)?,
                                schedule_year_date(Month::December, 17)?,
                                schedule_year_date(Month::December, 31)?,
                                schedule_year_date(Month::January, 14)?,
                                schedule_year_date(Month::January, 28)?,
                                schedule_year_date(Month::February, 11)?,
                                schedule_year_date(Month::February, 25)?,
                                schedule_year_date(Month::March, 10)?,
                                schedule_year_date(Month::March, 24)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on May 14, 28, Jun 11 & 25, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::May, 14)?,
                                schedule_year_date(Month::May, 28)?,
                                schedule_year_date(Month::June, 11)?,
                                schedule_year_date(Month::June, 25)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Apr 2, 16 & 30, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::April, 2)?,
                                schedule_year_date(Month::April, 16)?,
                                schedule_year_date(Month::April, 30)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Jul 9, 23, Aug 6, 20 & Sep 3, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::July, 9)?,
                                schedule_year_date(Month::July, 23)?,
                                schedule_year_date(Month::August, 6)?,
                                schedule_year_date(Month::August, 20)?,
                                schedule_year_date(Month::September, 3)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Sep 17 & Oct 1, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::September, 17)?,
                                schedule_year_date(Month::October, 1)?,
                            ])
                        }
                        "(DG) Dangerous Goods Sailing only on Oct 15, 29, Nov 12, 26, Dec 10, 24, Jan 7, 21, Feb 4, 18, Mar 3, 17 & 31, No other passengers permitted" => {
                            self.dangerous_goods_dates.only.extend([
                                schedule_year_date(Month::October, 15)?,
                                schedule_year_date(Month::October, 29)?,
                                schedule_year_date(Month::November, 12)?,
                                schedule_year_date(Month::November, 26)?,
                                schedule_year_date(Month::December, 10)?,
                                schedule_year_date(Month::December, 24)?,
                                schedule_year_date(Month::January, 7)?,
                                schedule_year_date(Month::January, 21)?,
                                schedule_year_date(Month::February, 4)?,
                                schedule_year_date(Month::February, 18)?,
                                schedule_year_date(Month::March, 3)?,
                                schedule_year_date(Month::March, 17)?,
                                schedule_year_date(Month::March, 31)?,
                            ])
                        }
                        "View dangerous goods sailings" => {}
                        "NOTES" => {}
                        _ => bail!("Unrecognized annotation text: {:?}", annotation_text),
                    }
                }
                annotation_is_exclamation = next_annotation_is_exclamation;
                Ok(())
            };
            inner().with_context(|| format!("Failed to parse annotation: {:?}", annotation_text.as_ref()))?;
        }
        Ok(())
    }
}
