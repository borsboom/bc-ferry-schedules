use crate::imports::*;
use crate::types::*;
use crate::utils::*;

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
    pub starstar: AnnotationDates,
    pub exclamation: AnnotationDates,
    pub exclamation_text: HashMap<&'static str, AnnotationDates>,
    pub exclamationexclamation_text: HashMap<&'static str, AnnotationDates>,
    pub hash: AnnotationDates,
    pub hash_text: HashMap<&'static str, AnnotationDates>,
    pub plus: AnnotationDates,
    pub plus_text: HashMap<&'static str, AnnotationDates>,
}

fn text_date_restriction<'a>(
    map: &'a mut HashMap<&'static str, AnnotationDates>,
    text: &'static str,
) -> &'a mut AnnotationDates {
    map.entry(text).or_insert_with(AnnotationDates::new)
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
        } else {
            DateRestriction::Except(self.except)
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
                if dr.is_never() {
                    None
                } else {
                    Some((k, dr))
                }
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
            starstar: AnnotationDates::new(),
            exclamation: AnnotationDates::new(),
            exclamation_text: HashMap::new(),
            exclamationexclamation_text: HashMap::new(),
            hash: AnnotationDates::new(),
            hash_text: HashMap::new(),
            plus: AnnotationDates::new(),
            plus_text: HashMap::new(),
        }
    }

    pub fn parse<T: AsRef<str>, I: IntoIterator<Item = T>>(
        &mut self,
        annotation_texts: I,
        effective_date_range: &DateRange,
    ) -> Result<()> {
        let schedule_year_date = |m, d| {
            effective_date_range
                .make_year_within(date(effective_date_range.from.year(), m, d))
                .context("Invalid date for schedule in annotation")
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
                        let parsed_date = NaiveDate::parse_from_str(
                            &format!("{} {}", date_text, effective_date_range.from.year()),
                            "%e %b %Y",
                        )
                        .with_context(|| format!("Failed to parse date: {:?}", date_text))?;
                        let date = effective_date_range.make_year_within(parsed_date).with_context(|| {
                            format!(
                                "Date is outside effective date range of schedule ({}): {:?}",
                                effective_date_range, parsed_date
                            )
                        })?;
                        dates_hashset.insert(date);
                    }
                } else {
                    let replaced_annotation_text = regex!(r"([!#*]*)\s*").replace(annotation_text.as_ref(), "$1 ");
                    let annotation_text = replaced_annotation_text.as_ref().trim();
                    // TODO: reduce repetition
                    match annotation_text {
                        "!" => next_annotation_is_exclamation = true,
                        "* On December 27, 2021, January 3 & February 21, 2022 the Holiday Monday schedule is in effect." | "* On December 27, 2021, January 3 and February 21, 2022 the Holiday Monday schedule is in effect." => {
                            let dates = [
                                date(2021, 12, 27),
                                date(2022, 1, 3),
                                date(2022, 2, 21),
                            ];
                            self.holiday_monday.only.extend(dates);
                            self.star.except.extend(dates);
                        }
                        "* On December 27, 2021, January 3, February 21 & April 18, 2022 the Holiday Monday schedule is in effect." => {
                            let dates = [
                                date(2021, 12, 27),
                                date(2022, 1, 3),
                                date(2022, 2, 21),
                                date(2022, 4, 18),
                            ];
                            self.holiday_monday.only.extend(dates);
                            self.star.except.extend(dates);
                        }
                        "** Except on December 25, 2021 & January 1, 2022." =>
                            self.starstar.except.extend([
                                date(2021, 12, 25),
                                date(2022, 1, 1),
                            ]),
                        "* Except On December 27, 2021, January 3 & February 21, 2022." =>
                            self.star.except.extend([
                                date(2021, 12, 27),
                                date(2022, 1, 3),
                                date(2022, 2, 21),
                            ]),
                        "* Except on April 14th" => {
                            self.star.except.extend([schedule_year_date(4, 14)?]);
                        }
                        "** Except on April 14th" => {
                            self.starstar.except.extend([schedule_year_date(4, 14)?]);
                        }
                        "* Except on April 18th" => {
                            self.star.except.extend([schedule_year_date(4, 18)?]);
                        }
                        "** Only on April 14th" => {
                            self.starstar.only.extend([schedule_year_date(4, 14)?]);
                        }
                        "* Only on April 14th" => {
                            self.star.only.extend([schedule_year_date(4, 14)?]);
                        }
                        "On April 18th the Holiday Monday schedule is in effect." if annotation_is_exclamation => {
                            let dates = [schedule_year_date(4, 18)?];
                            self.holiday_monday.only.extend(dates);
                            self.exclamation.except.extend(dates);
                        }
                        "* On April 18th the Holiday Monday schedule is in effect." => {
                            let dates = [schedule_year_date(4, 18)?];
                            self.holiday_monday.only.extend(dates);
                            self.star.except.extend(dates);
                        }
                        "* On May 23rd the Holiday Monday schedule is in effect." => {
                            let dates = [schedule_year_date(5, 23)?];
                            self.holiday_monday.only.extend(dates);
                            self.star.except.extend(dates);
                        }
                        "** Except February 14 to March 28, 2022." =>
                            self.starstar.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28)
                                }.iter_days()),
                        "! Except February 14 to March 28, 2022." =>
                            self.exclamation.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28)
                                }.iter_days()),
                        "# Except February 14-March 28, 2022" =>
                            self.hash.except.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28)
                                }.iter_days()),
                        "** February 14-March 28, 2022 only" =>
                            self.starstar.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28)
                                }.iter_days()),
                        "# February 14 to March 28, 2022 only." =>
                            self.hash.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28)
                                }.iter_days()),
                        "+ February 14 to March 28, 2022 only." =>
                            self.plus.only.extend(
                                DateRange {
                                    from: date(2022, 2, 14),
                                    to: date(2022, 3, 28)
                                }.iter_days()),
                        "# Only on February 18, 25, March 4, 11, 18, 25, 2022." | "# Only on February 18, 25, March 4, 11, 18 and 25, 2022." | "# February 18, 25, March 4, 11, 18 and 25, 2022 only." =>
                            self.hash.only.extend([
                                date(2022, 2, 18),
                                date(2022, 2, 25),
                                date(2022, 3, 4),
                                date(2022, 3, 11),
                                date(2022, 3, 18),
                                date(2022, 3, 25)
                            ]),
                        "** Except February 18, 25, March 4, 11, 18 and 25, 2022." | "** Except on February 18, 25, March 4, 11, 18 and 25, 2022." =>
                            self.starstar.except.extend([
                                date(2022, 2, 18),
                                date(2022, 2, 25),
                                date(2022, 3, 4),
                                date(2022, 3, 11),
                                date(2022, 3, 18),
                                date(2022, 3, 25)
                            ]),
                        "** Except May 8, 22 & June 5, 19" =>
                            self.starstar.except.extend([
                                schedule_year_date(5, 8)?,
                                schedule_year_date(5, 22)?,
                                schedule_year_date(6, 5)?,
                                schedule_year_date(6, 19)?,
                            ]),
                        "** Except May 8, 29 & June 5, 19" =>
                            self.starstar.except.extend([
                                schedule_year_date(5, 8)?,
                                schedule_year_date(5, 29)?,
                                schedule_year_date(6, 5)?,
                                schedule_year_date(6, 19)?,
                            ]),
                        "** Except on May 15, 29 & June 12, 26" =>
                            self.starstar.except.extend([
                                schedule_year_date(5, 15)?,
                                schedule_year_date(5, 29)?,
                                schedule_year_date(6, 12)?,
                                schedule_year_date(6, 26)?,
                            ]),
                        "# Foot passengers only on this sailing - Vehicles permitted February 14 to March 28, 2022." =>
                            //TODO: Update the front-end to display text annotations
                            text_date_restriction(&mut self.hash_text, "Foot passengers only").except.extend(DateRange {
                                from: schedule_year_date(2, 14)?,
                                to: schedule_year_date(3, 28)?,
                            }.iter_days()),
                        "# Foot passengers only February 14 to March 28." =>
                            text_date_restriction(&mut self.hash_text, "Foot passengers only").only.extend(DateRange {
                                    from: schedule_year_date(2, 14)?,
                                    to: schedule_year_date(3, 28)?,
                                }.iter_days()),
                        "# Foot passengers only on this sailing." => {
                            text_date_restriction(&mut self.hash_text, "Foot passengers only");
                        }
                        "+ Foot passengers only Fridays February 18, 25, March 4, 11, 18, 25." =>
                            text_date_restriction(&mut self.plus_text, "Foot passengers only").only.extend([
                                schedule_year_date(2, 18)?,
                                schedule_year_date(2, 25)?,
                                schedule_year_date(3, 4)?,
                                schedule_year_date(3, 11)?,
                                schedule_year_date(3, 18)?,
                                schedule_year_date(3, 25)?,
                            ]),
                        "! Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing." => {
                            text_date_restriction(&mut self.exclamation_text, "Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing");
                        }
                        "!! On February 18, 25, March 4, 11, 18, 25 arrival time will be 5:35 PM." => {
                            text_date_restriction(&mut self.exclamationexclamation_text, "Arrival time will be 5:35 PM").only.extend([
                                schedule_year_date(2, 18)?,
                                schedule_year_date(2, 25)?,
                                schedule_year_date(3, 4)?,
                                schedule_year_date(3, 11)?,
                                schedule_year_date(3, 18)?,
                                schedule_year_date(3, 25)?,
                            ])
                        }
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
