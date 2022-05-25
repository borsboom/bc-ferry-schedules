use crate::imports::*;
use crate::macros::*;

#[derive(Clone, Debug)]
pub struct AnnotationDates {
    pub only: HashSet<NaiveDate>,
    pub except: HashSet<NaiveDate>,
}

#[derive(Debug)]
pub struct Annotations {
    // TODO: reduce repetition
    pub holiday_monday: AnnotationDates,
    pub star: AnnotationDates,
    pub star_by_time: HashMap<NaiveTime, AnnotationDates>,
    pub star2: AnnotationDates,
    pub star3: AnnotationDates,
    pub exclamation: AnnotationDates,
    pub exclamation_text: HashMap<Cow<'static, str>, AnnotationDates>,
    pub exclamation2_text: HashMap<Cow<'static, str>, AnnotationDates>,
    pub hash: AnnotationDates,
    pub hash_text: HashMap<Cow<'static, str>, AnnotationDates>,
    pub plus: AnnotationDates,
    pub plus_text: HashMap<Cow<'static, str>, AnnotationDates>,
}

fn text_date_restriction<'a, T: Into<Cow<'static, str>>>(
    map: &'a mut HashMap<Cow<'static, str>, AnnotationDates>,
    text: T,
) -> &'a mut AnnotationDates {
    map.entry(text.into()).or_insert_with(AnnotationDates::new)
}

impl AnnotationDates {
    pub fn new() -> AnnotationDates {
        AnnotationDates { only: HashSet::new(), except: HashSet::new() }
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
        F: Fn(&NaiveDate) -> bool,
    {
        self.only.retain(&predicate);
        self.except.retain(&predicate);
        self.into_date_restriction()
    }

    pub fn into_date_restriction_by_weekday(self, weekday: Weekday) -> DateRestriction {
        self.into_date_restriction_by(|d| d.weekday() == weekday)
    }

    pub fn into_date_restriction_by_weekdays(self, weekdays: &HashMap<Weekday, DateRestriction>) -> DateRestriction {
        self.into_date_restriction_by(|date: &NaiveDate| {
            weekdays.get(&date.weekday()).map(|dr| dr.includes_date(*date)).unwrap_or(false)
        })
    }

    pub fn map_to_date_restrictions_by_weekdays<K: Eq + Hash>(
        map: HashMap<K, AnnotationDates>,
        weekdays: &HashMap<Weekday, DateRestriction>,
    ) -> HashMap<K, DateRestriction> {
        map.into_iter()
            .filter_map(|(k, ad)| {
                let dr = ad.into_date_restriction_by_weekdays(weekdays);
                (!dr.is_never()).then(|| (k, dr))
            })
            .collect()
    }
}

impl Annotations {
    pub fn new() -> Annotations {
        Annotations {
            holiday_monday: AnnotationDates::new(),
            star: AnnotationDates::new(),
            star_by_time: HashMap::new(),
            star2: AnnotationDates::new(),
            star3: AnnotationDates::new(),
            exclamation: AnnotationDates::new(),
            exclamation_text: HashMap::new(),
            exclamation2_text: HashMap::new(),
            hash: AnnotationDates::new(),
            hash_text: HashMap::new(),
            plus: AnnotationDates::new(),
            plus_text: HashMap::new(),
        }
    }

    fn star_holiday_monday_extend(&mut self, dates: &[NaiveDate]) {
        self.holiday_monday.only.extend(dates);
        self.star.except.extend(dates);
    }

    pub fn parse<T: AsRef<str>, I: IntoIterator<Item = T>>(
        &mut self,
        annotation_texts: I,
        date_range: &DateRange,
    ) -> Result<()> {
        let from_year = date_range.from.year();
        let to_year = date_range.to.year();
        let schedule_year_date = |m, d| {
            date_range.make_year_within(date(from_year, m, d)).context("Invalid date for schedule in annotation")
        };
        let mut annotation_is_exclamation = false;
        for annotation_text in annotation_texts {
            let mut inner = || {
                let mut next_annotation_is_exclamation = false;
                if let Some(captures) =
                    regex!(r"\*(\d+:\d+ [AP]M) (Not Available|Only) on: (.*)\*").captures(annotation_text.as_ref())
                {
                    let time_text = &captures[1];
                    let time = NaiveTime::parse_from_str(time_text, "%l:%M %p")
                        .with_context(|| format!("Failed to parse time: {:?}", time_text))?;
                    let dates = self.star_by_time.entry(time).or_insert_with(AnnotationDates::new);
                    let dates_hashset = match &captures[2] {
                        "Not Available" => &mut dates.except,
                        "Only" => &mut dates.only,
                        other => bail!("Expected \"Only\" or \"Not Available\" in: {:?}", other),
                    };
                    for date_text in captures[3].split(',').map(|s| s.trim()) {
                        let parsed_date =
                            NaiveDate::parse_from_str(&format!("{} {}", date_text, from_year), "%e %b %Y")
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
                        "* On December 27, 2021, January 3 & February 21, 2022 the Holiday Monday schedule is in effect" |
                        "* On December 27, 2021, January 3 and February 21, 2022 the Holiday Monday schedule is in effect" =>
                            self.star_holiday_monday_extend(&[
                                date(2021, 12, 27),
                                date(2022, 1, 3),
                                date(2022, 2, 21),
                            ]),
                        "* On December 27, 2021, January 3, February 21 & April 18, 2022 the Holiday Monday schedule is in effect" =>
                            self.star_holiday_monday_extend(&[
                                date(2021, 12, 27),
                                date(2022, 1, 3),
                                date(2022, 2, 21),
                                date(2022, 4, 18),
                            ]),
                        "* On April 18, 2022 the Holiday Monday Schedule is in effect" |
                        "* On April 18, 2022, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[
                                date(2022, 4, 18),
                            ]),
                        "* On April 18th, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[
                                schedule_year_date(4, 18)?,
                            ]),
                        "* On May 23rd, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On May 23, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On May 23rd the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm." =>
                            self.star_holiday_monday_extend(&[
                                schedule_year_date(5, 23)?,
                            ]),
                        "** Except on December 25, 2021 & January 1, 2022" =>
                            self.star2.except.extend([
                                date(2021, 12, 25),
                                date(2022, 1, 1),
                            ]),
                        "* Except On December 27, 2021, January 3 & February 21, 2022" =>
                            self.star.except.extend([
                                date(2021, 12, 27),
                                date(2022, 1, 3),
                                date(2022, 2, 21),
                            ]),
                        "* Except On December 26, 2022, January 2 & February 20, 2023" =>
                            self.star.except.extend([
                                date(2022, 12, 26),
                                date(2023, 1, 2),
                                date(2023, 2, 20),
                            ]),
                        "* Except on August 1st and September 5th 2022" =>
                            self.star.except.extend([
                                date(2022, 8, 1),
                                date(2022, 9, 5),
                            ]),
                        "* Except on October 10, 2022" =>
                            self.star.except.extend([
                                date(2022, 10, 10),
                            ]),
                        "* Except on April 14th" =>
                            self.star.except.extend([schedule_year_date(4, 14)?]),
                        "** Except on April 14th" =>
                            self.star2.except.extend([schedule_year_date(4, 14)?]),
                        "* Except on April 18th" =>
                            self.star.except.extend([schedule_year_date(4, 18)?]),
                        "** Except on July 3, 17, 31 August 14, 28" |
                        "** Except on Jul 3, 17, 31, Aug 14 & 28" =>
                            self.star2.except.extend([
                                schedule_year_date(7, 3)?,
                                schedule_year_date(7, 17)?,
                                schedule_year_date(7, 31)?,
                                schedule_year_date(8, 14)?,
                                schedule_year_date(8, 28)?
                            ]),
                        "** Except on July 10, 24, August 7, 21, September 4" |
                        "** Except on Jul 10, 24, Aug 7, 21 & Sep 4" =>
                            self.star2.except.extend([
                                schedule_year_date(7, 10)?,
                                schedule_year_date(7, 24)?,
                                schedule_year_date(8, 7)?,
                                schedule_year_date(8, 21)?,
                                schedule_year_date(9, 4)?
                            ]),
                        "** Except Sep 11, 25 & Oct 9" =>
                            self.star2.except.extend([
                                schedule_year_date(9, 11)?,
                                schedule_year_date(9, 25)?,
                                schedule_year_date(10, 9)?
                            ]),
                        "** Except on September 18 and October 2" |
                        "** Except Sep 18 & Oct 2" |
                        "** Except on Sep 18 & Oct 2" =>
                            self.star2.except.extend([schedule_year_date(9, 18)?, schedule_year_date(10, 2)?]),
                        "*** Except Sep 18 & Oct 2" =>
                            self.star3.except.extend([schedule_year_date(9, 18)?, schedule_year_date(10, 2)?]),
                        "** Only on April 14th" =>
                            self.star2.only.extend([schedule_year_date(4, 14)?]),
                        "** Only on December 23 and December 30" |
                        "** Only on Dec 23 & 30" =>
                            self.star2.only.extend([schedule_year_date(12, 23)?, schedule_year_date(12, 30)?]),
                        "** Only on July 3, 17, 31 August 14, 28" |
                        "** Only on July 3, 17, 31, August 14, 28" |
                        "** Only on Jul 3, 17, 31 Aug 14 & 28" |
                        "** Only on Jul 3, 17, 31, Aug 14 & 28" =>
                            self.star2.only.extend([
                                schedule_year_date(7, 3)?,
                                schedule_year_date(7, 17)?,
                                schedule_year_date(7, 31)?,
                                schedule_year_date(8, 14)?,
                                schedule_year_date(8, 28)?
                            ]),
                        "** Only on September 18 and October 2" |
                        "** Only on Sep 18 & Oct 2" =>
                            self.star2.only.extend([schedule_year_date(9, 18)?, schedule_year_date(10, 2)?]),
                        "** Only on September 11, 25, October 9" =>
                            self.star2.only.extend([
                                schedule_year_date(9, 11)?,
                                schedule_year_date(9, 25)?,
                                schedule_year_date(10, 9)?
                            ]),
                        "*** Only on July 10, 24, August 7, 21, September 4" |
                        "*** Only on Jul 10, 24, Aug 7, 21 & Sep 4" =>
                            self.star3.only.extend([
                                schedule_year_date(7, 10)?,
                                schedule_year_date(7, 24)?,
                                schedule_year_date(8, 7)?,
                                schedule_year_date(8, 21)?,
                                schedule_year_date(9, 4)?
                            ]),
                        "*** Only on September 11, 25, October 9" |
                        "*** Only on Sep 11, 25 & Oct 9" =>
                            self.star3.only.extend([
                                schedule_year_date(9, 11)?,
                                schedule_year_date(9, 25)?,
                                schedule_year_date(10, 9)?
                            ]),
                        "* Only on April 14th" =>
                            self.star.only.extend([schedule_year_date(4, 14)?]),
                        "On April 18th the Holiday Monday schedule is in effect" if annotation_is_exclamation =>
                            self.star_holiday_monday_extend(&[schedule_year_date(4, 18)?]),
                        "* On April 18th the Holiday Monday schedule is in effect" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(4, 18)?]),
                        "* On May 23rd the Holiday Monday schedule is in effect" |
                        "* On May 23 the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(5, 23)?]),
                        "* On August 1st and September 5th 2022, the Holiday Monday schedule is in effect" |
                        "* On August 1st and September 5th 2022, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[date(2022, 8, 1), date(2022, 9, 5)]),
                        "* On Aug 1 & Sep 5, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(8, 1)?, schedule_year_date(9, 5)?]),
                        "* On October 10, 2022, the Holiday Monday Schedule is in effect" |
                        "* On October 10, 2022, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On Oct 10th the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On Oct 10, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[schedule_year_date(10, 10)?]),
                        "* On December 26, 2022, January 2 & February 20, 2023 the Holiday Monday schedule is in effect" |
                        "* On December 26, 2022, January 2 & February 20, 2023, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" |
                        "* On Dec 26, 2022, Jan 2 & Feb 20, 2023, the Monday schedule is in effect until 2:00 pm, the Holiday Monday Schedule is in effect after 2:00 pm" =>
                            self.star_holiday_monday_extend(&[date(2022, 12, 26), date(2023, 1, 2), date(2023, 2, 20)]),
                        "** Except February 14 to March 28, 2022" =>
                            self.star2.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28),
                                }.iter_days()),
                        "! Except February 14 to March 28, 2022" =>
                            self.exclamation.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28),
                                }.iter_days()),
                        "! Except February 14 to March 16, 2022" =>
                            self.exclamation.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 16),
                                }.iter_days()),
                        "# Except February 14-March 28, 2022" =>
                            self.hash.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28),
                                }.iter_days()),
                        "# Except February 14-March 16, 2022" =>
                            self.hash.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 16),
                                }.iter_days()),
                        "** February 14-March 28, 2022 only" =>
                            self.star2.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28),
                                }.iter_days()),
                        "** February 14-March 16, 2022 only" =>
                            self.star2.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 16),
                                }.iter_days()),
                        "# February 14 to March 28, 2022 only" =>
                            self.hash.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28),
                                }.iter_days()),
                        "# February 14 to March 16, 2022 only" =>
                            self.hash.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 16),
                                }.iter_days()),
                        "+ February 14 to March 28, 2022 only" =>
                            self.plus.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28),
                                }.iter_days()),
                        "+ February 14 to March 16, 2022 only" =>
                            self.plus.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 16),
                                }.iter_days()),
                        "# Only on February 18, 25, March 4, 11, 18, 25, 2022" |
                        "# Only on February 18, 25, March 4, 11, 18 and 25, 2022" | "# February 18, 25, March 4, 11, 18 and 25, 2022 only" =>
                            self.hash.only.extend([
                                date(2022, 2, 18),
                                date(2022, 2, 25),
                                date(2022, 3, 4),
                                date(2022, 3, 11),
                                date(2022, 3, 18),
                                date(2022, 3, 25),
                            ]),
                        "# Only on February 18, 25, March 4, 11, 2022" |
                        "# February 18, 25, March 4, 11, 2022 only" =>
                            self.hash.only.extend([
                                date(2022, 2, 18),
                                date(2022, 2, 25),
                                date(2022, 3, 4),
                                date(2022, 3, 11),
                            ]),
                        "** Except February 18, 25, March 4, 11, 18 and 25, 2022" |
                        "** Except on February 18, 25, March 4, 11, 18 and 25, 2022" =>
                            self.star2.except.extend([
                                date(2022, 2, 18),
                                date(2022, 2, 25),
                                date(2022, 3, 4),
                                date(2022, 3, 11),
                                date(2022, 3, 18),
                                date(2022, 3, 25),
                            ]),
                        "** Except February 18, 25, March 4, 11, 2022" |
                        "** Except on February 18, 25, March 4, 11, 2022" =>
                            self.star2.except.extend([
                                date(2022, 2, 18),
                                date(2022, 2, 25),
                                date(2022, 3, 4),
                                date(2022, 3, 11),
                            ]),
                        "** Except February 14 to March 16, 2022" =>
                            self.star2.except.extend(DateRange {
                                from: date(2022, 2, 14),
                                to: date(2022, 3, 16)
                            }.iter_days()),
                        "** Except May 8, 22 & June 5, 19" |
                        "** Except May 8, 22, Jun 5 & 19" =>
                            self.star2.except.extend([
                                schedule_year_date(5, 8)?,
                                schedule_year_date(5, 22)?,
                                schedule_year_date(6, 5)?,
                                schedule_year_date(6, 19)?,
                            ]),
                        "** Except May 8, 29 & June 5, 19" =>
                            self.star2.except.extend([
                                schedule_year_date(5, 8)?,
                                schedule_year_date(5, 29)?,
                                schedule_year_date(6, 5)?,
                                schedule_year_date(6, 19)?,
                            ]),
                        "** Except on May 15, 29 & June 12, 26" |
                        "** Except May 15, 29, Jun 12 & 26" |
                        "** Except on May 15, 29, Jun 12 & 26" =>
                            self.star2.except.extend([
                                schedule_year_date(5, 15)?,
                                schedule_year_date(5, 29)?,
                                schedule_year_date(6, 12)?,
                                schedule_year_date(6, 26)?,
                            ]),
                        "*** Except May 15, 29, Jun 12 & 26" =>
                            self.star3.except.extend([
                                schedule_year_date(5, 15)?,
                                schedule_year_date(5, 29)?,
                                schedule_year_date(6, 12)?,
                                schedule_year_date(6, 26)?,
                            ]),
                        "*** Only on May 8, 22, Jun 12 & 26" =>
                            self.star3.except.extend([
                                schedule_year_date(5, 8)?,
                                schedule_year_date(5, 22)?,
                                schedule_year_date(6, 12)?,
                                schedule_year_date(6, 26)?,
                            ]),
                        "# Foot passengers only on this sailing - Vehicles permitted February 14 to March 28, 2022" =>
                            text_date_restriction(&mut self.hash_text, "Foot passengers only").except.extend(DateRange {
                                from: schedule_year_date(2, 14)?,
                                to: schedule_year_date(3, 28)?,
                            }.iter_days()),
                        "# Foot passengers only on this sailing - Vehicles permitted February 14 to March 16, 2022" =>
                            text_date_restriction(&mut self.hash_text, "Foot passengers only").except.extend(DateRange {
                                from: schedule_year_date(2, 14)?,
                                to: schedule_year_date(3, 16)?,
                            }.iter_days()),
                        "# Foot passengers only February 14 to March 28" =>
                            text_date_restriction(&mut self.hash_text, "Foot passengers only").only.extend(DateRange {
                                    from: schedule_year_date(2, 14)?,
                                    to: schedule_year_date(3, 28)?,
                                }.iter_days()),
                        "# Foot passengers only February 14 to March 16" =>
                            text_date_restriction(&mut self.hash_text, "Foot passengers only").only.extend(DateRange {
                                    from: schedule_year_date(2, 14)?,
                                    to: schedule_year_date(3, 16)?,
                                }.iter_days()),
                        "+ Foot passengers only through March 28" if to_year == 2022 =>
                            text_date_restriction(&mut self.plus_text, "Foot passengers only").only.extend(DateRange {
                                from: date(2022, 2, 14),
                                to: date(2022, 3, 28),
                            }.iter_days()),
                        "+ Foot passengers only through March 16" if to_year == 2022 =>
                            text_date_restriction(&mut self.plus_text, "Foot passengers only").only.extend(DateRange {
                                from: date(2022, 2, 14),
                                to: date(2022, 3, 16),
                            }.iter_days()),
                        "# Foot passengers only on this sailing" => {
                            text_date_restriction(&mut self.hash_text, "Foot passengers only");
                        }
                        "# Foot passengers only on this sailing except the 9:10 AM sailing on May 30 which will permit vehicles" => {
                            text_date_restriction(
                                &mut self.hash_text,
                                "Foot passengers only on this sailing except the 9:10 AM sailing on May 30 which will permit vehicles"
                            );
                        }
                        "+ Foot passengers only Fridays February 18, 25, March 4, 11, 18, 25" =>
                            text_date_restriction(&mut self.plus_text, "Foot passengers only").only.extend([
                                schedule_year_date(2, 18)?,
                                schedule_year_date(2, 25)?,
                                schedule_year_date(3, 4)?,
                                schedule_year_date(3, 11)?,
                                schedule_year_date(3, 18)?,
                                schedule_year_date(3, 25)?,
                            ]),
                        "! Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing" => {
                            text_date_restriction(
                                &mut self.exclamation_text,
                                "Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing"
                            );
                        }
                        "!! On February 18, 25, March 4, 11, 18, 25 arrival time will be 5:35 PM" => {
                            text_date_restriction(&mut self.exclamation2_text, "Arrival time will be 5:35 PM").only.extend([
                                schedule_year_date(2, 18)?,
                                schedule_year_date(2, 25)?,
                                schedule_year_date(3, 4)?,
                                schedule_year_date(3, 11)?,
                                schedule_year_date(3, 18)?,
                                schedule_year_date(3, 25)?,
                            ])
                        }
                        "!! On February 18, 25, March 4, 11 arrival time will be 5:35 PM" => {
                            text_date_restriction(&mut self.exclamation2_text, "Arrival time will be 5:35 PM").only.extend([
                                schedule_year_date(2, 18)?,
                                schedule_year_date(2, 25)?,
                                schedule_year_date(3, 4)?,
                                schedule_year_date(3, 11)?,
                            ])
                        }
                        "View dangerous goods sailings" => {}
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
