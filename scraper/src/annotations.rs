use crate::imports::*;
use crate::types::*;
use crate::utils::*;

#[derive(Clone, Debug)]
pub struct DateRestriction {
    pub only: HashSet<NaiveDate>,
    pub except: HashSet<NaiveDate>,
}

#[derive(Debug)]
pub struct Annotations {
    pub holiday_monday: DateRestriction,
    pub star: DateRestriction,
    pub starstar: DateRestriction,
    pub exclamation: DateRestriction,
    pub exclamation_text: Vec<&'static str>,
    pub hash: DateRestriction,
    pub hash_text: Vec<&'static str>,
    pub plus: DateRestriction,
    pub plus_text: Vec<&'static str>,
}

impl DateRestriction {
    pub fn new() -> DateRestriction {
        DateRestriction { only: HashSet::new(), except: HashSet::new() }
    }

    pub fn extend(&mut self, other: &DateRestriction) {
        self.except.extend(&other.except);
        self.only.extend(&other.only);
    }

    pub fn normalize(&mut self) {
        let common_dates = self.except.intersection(&self.only).cloned().collect::<Vec<_>>();
        for common_date in common_dates {
            self.except.remove(&common_date);
            self.only.remove(&common_date);
        }
    }
}

impl Annotations {
    pub fn new() -> Annotations {
        Annotations {
            holiday_monday: DateRestriction::new(),
            star: DateRestriction::new(),
            starstar: DateRestriction::new(),
            exclamation: DateRestriction::new(),
            exclamation_text: Vec::new(),
            hash: DateRestriction::new(),
            hash_text: Vec::new(),
            plus: DateRestriction::new(),
            plus_text: Vec::new(),
        }
    }

    pub fn parse(&mut self, annotation_texts: &[String], effective_date_range: &DateRange) -> Result<()> {
        let mut annotation_is_exclamation = false;
        for annotation_text in annotation_texts {
            let mut next_annotation_is_exclamation = false;
            let replaced_annotation_text = regex!(r"([!#*]*)\s*").replace(annotation_text, "$1 ");
            let annotation_text = replaced_annotation_text.as_ref().trim();
            match annotation_text {
                "!" => next_annotation_is_exclamation = true,
                "* On December 27, 2021, January 3 & February 21, 2022 the Holiday Monday schedule is in effect." | "* On December 27, 2021, January 3 and February 21, 2022 the Holiday Monday schedule is in effect." => {
                    let dates = [
                        date(2021, 12, 27),
                        date(2022, 1, 3),
                        date(2022, 2, 21),
                    ];
                    self.holiday_monday.except.extend(dates);
                    self.star.except.extend(dates);
                }
                "* On December 27, 2021, January 3, February 21 & April 18, 2022 the Holiday Monday schedule is in effect." => {
                    let dates = [
                        date(2021, 12, 27),
                        date(2022, 1, 3),
                        date(2022, 2, 21),
                        date(2022, 4, 18),
                    ];
                    self.holiday_monday.except.extend(dates);
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
                "* Except on April 14th" if effective_date_range.from.year() == 2022 => {
                    self.star.except.extend([date(2022, 4, 14)]);
                }
                "** Except on April 14th" if effective_date_range.from.year() == 2022 => {
                    self.starstar.except.extend([date(2022, 4, 14)]);
                }
                "* Except on April 18th" if effective_date_range.from.year() == 2022 => {
                    self.star.except.extend([date(2022, 4, 18)]);
                }
                "** Only on April 14th" if effective_date_range.from.year() == 2022 => {
                    self.starstar.only.extend([date(2022, 4, 14)]);
                }
                "* Only on April 14th" if effective_date_range.from.year() == 2022 => {
                    self.star.only.extend([date(2022, 4, 14)]);
                }
                "On April 18th the Holiday Monday schedule is in effect." if annotation_is_exclamation && effective_date_range.from.year() == 2022 => {
                    let dates = [date(2022, 4, 18)];
                    self.holiday_monday.only.extend(dates);
                    self.exclamation.except.extend(dates);
                }
                "* On April 18th the Holiday Monday schedule is in effect." if effective_date_range.from.year() == 2022 => {
                    let dates = [date(2022, 4, 18)];
                    self.holiday_monday.only.extend(dates);
                    self.star.except.extend(dates);
                }
                "* On May 23rd the Holiday Monday schedule is in effect." if effective_date_range.from.year() == 2022 => {
                    let dates = [date(2022, 5, 23)];
                    self.holiday_monday.only.extend(dates);
                    self.star.except.extend(dates);
                }
                "** Except February 14 to March 28, 2022." =>
                    self.starstar.except.extend(
                        DateRange {
                            from: date(2022, 2, 14),
                            to: date(2022, 3, 28)
                        }.iter_days()),
                "# Only on February 18, 25, March 4, 11, 18, 25, 2022." | "# Only on February 18, 25, March 4, 11, 18 and 25, 2022." =>
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
                "** Except May 8, 22 & June 5, 19"  if effective_date_range.from.year() == 2022 =>
                    self.starstar.except.extend([
                        date(2022, 5, 8),
                        date(2022, 5, 22),
                        date(2022, 6, 5),
                        date(2022, 6, 19),
                    ]),
                "** Except May 8, 29 & June 5, 19"  if effective_date_range.from.year() == 2022 =>
                    self.starstar.except.extend([
                        date(2022, 5, 8),
                        date(2022, 5, 29),
                        date(2022, 6, 5),
                        date(2022, 6, 19),
                    ]),
                "** Except on May 15, 29 & June 12, 26"  if effective_date_range.from.year() == 2022 =>
                    self.starstar.except.extend([
                        date(2022, 5, 15),
                        date(2022, 5, 29),
                        date(2022, 6, 12),
                        date(2022, 6, 26),
                    ]),
                "# Foot passengers only on this sailing - Vehicles permitted February 14 to March 28, 2022." =>
                    //TODO: For these kinds of notes, encode the date restriction
                    //TODO: Update the front-end to display text annotations (and put them to DynamoDB)
                    self.hash_text.push("Foot passengers only on this sailing - Vehicles permitted February 14 to March 28, 2022."),
                "# Foot passengers only February 14 to March 28." =>
                    self.hash_text.push("Foot passengers only February 14 to March 28."),
                "# Foot passengers only on this sailing." =>
                    self.hash_text.push("Foot passengers only on this sailing"),
                "+ Foot passengers only Fridays February 18, 25, March 4, 11, 18, 25." =>
                    self.plus_text.push("Foot passengers only Fridays February 18, 25, March 4, 11, 18, 25."),
                "! Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing." =>
                    self.exclamation_text.push("Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing."),
                _ => bail!("Unrecognized annotation text: {:?}", annotation_text),
            }
            annotation_is_exclamation = next_annotation_is_exclamation;
        }
        Ok(())
    }
}
