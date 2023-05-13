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
    pub dg_dates: AnnotationDates,
    pub is_dg_only: bool,
    pub star_dates: AnnotationDates,
    pub star_dates_by_time: HashMap<Time, AnnotationDates>,
    pub all_dates: AnnotationDates,
    pub all_notes: AnnotationNotes,
}

fn text_date_restriction<T: Into<Cow<'static, str>>>(notes: &mut AnnotationNotes, text: T) -> &mut AnnotationDates {
    notes.map.entry(text.into()).or_insert_with(AnnotationDates::new)
}

pub fn annotation_notes_date_restictions(
    row_notes: AnnotationNotes,
    weekday: Weekday,
    date_restriction: &DateRestriction,
) -> HashMap<Cow<'static, str>, DateRestriction> {
    AnnotationDates::map_to_date_restrictions_by_weekday(row_notes.map.into_iter(), weekday, date_restriction)
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
        F: Fn(&Date) -> bool,
    {
        self.only.retain(&predicate);
        self.except.retain(&predicate);
        self.into_date_restriction()
    }

    pub fn into_date_restriction_by_weekday(self, weekday: Weekday) -> DateRestriction {
        self.into_date_restriction_by(|date| date.weekday() == weekday)
    }

    pub fn into_date_restriction_by_weekday_and_date_restriction(
        self,
        weekday: Weekday,
        date_restriction: &DateRestriction,
    ) -> DateRestriction {
        self.into_date_restriction_by(|date| date.weekday() == weekday && date_restriction.includes_date(*date))
    }

    pub fn map_to_date_restrictions_by_weekday<I, K>(
        map: I,
        weekday: Weekday,
        date_restriction: &DateRestriction,
    ) -> HashMap<K, DateRestriction>
    where
        K: Eq + Hash,
        I: IntoIterator<Item = (K, AnnotationDates)>,
    {
        map.into_iter()
            .filter_map(|(k, ad)| {
                let dr = ad.into_date_restriction_by_weekday_and_date_restriction(weekday, date_restriction);
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
    pub fn new() -> Annotations {
        Annotations {
            dg_dates: AnnotationDates::new(),
            is_dg_only: false,
            star_dates: AnnotationDates::new(),
            star_dates_by_time: HashMap::new(),
            all_dates: AnnotationDates::new(),
            all_notes: AnnotationNotes::new(),
        }
    }

    pub fn parse<T: AsRef<str>, I: IntoIterator<Item = T>>(
        &mut self,
        annotation_texts: I,
        date_range: &DateRange,
    ) -> Result<()> {
        for annotation_text in annotation_texts {
            let mut inner = || {
                let annotation_text = regex!(r"\.$").replace(annotation_text.as_ref(), "");
                let annotation_text = regex!(r"(?i)\bApril\b").replace_all(annotation_text.as_ref(), "Apr");
                let annotation_text = regex!(r", \d{4}\b").replace_all(annotation_text.as_ref(), "");
                let annotation_text = regex!(r"(?i)( & |, and | and )").replace_all(annotation_text.as_ref(), ", ");
                let annotation_text =
                    regex!(r"(?i)\b([a-z]{3})(\d{1,2})\b").replace_all(annotation_text.as_ref(), "$1 $2");
                let annotation_text = regex!(r"(?i)\b([a-z]{3} \d{1,2}) ([a-z]{3} \d{1,2})\b")
                    .replace_all(annotation_text.as_ref(), "$1, $2");
                let annotation_text = regex!(r"(?i)\b([a-z]{3}) (\d{1,2}),? (\d{1,2}),? (\d{1,2})\b")
                    .replace_all(annotation_text.as_ref(), "$1 $2, $1 $3, $1 $4");
                let annotation_text = regex!(r"(?i)\b([a-z]{3}) (\d{1,2}),? (\d{1,2})\b")
                    .replace_all(annotation_text.as_ref(), "$1 $2, $1 $3");
                let annotation_text = regex!(r"(?i)^([a-z]{3} \d{1,2})(, [a-z]{3} \d{1,2})* only$")
                    .replace(annotation_text.as_ref(), "Only $1$2");
                let annotation_text = regex!(r"(?i)^(DG Sailing only .*), no other passengers permitted$")
                    .replace(annotation_text.as_ref(), "$1");
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
                        let date_within_range = date_range
                            .parse_date_within(
                                date_text,
                                format_description!(
                                    "[day padding:none] [month repr:short case_sensitive:false] [year]"
                                ),
                            )
                            .with_context(|| {
                                format!("Failed to parse sailing date {:?} in {:?}", date_text, annotation_text)
                            })?;
                        if let Some(date) = date_within_range {
                            dates_hashset.insert(date);
                        } else {
                            warn!("Date is outside date range of schedule ({}): {:?}", date_range, date_text);
                        }
                    }
                } else if let Some(captures) =
                    regex!(r"(?i)^(Except|Only|DG Sailing only)( on)? (.*)").captures(annotation_text.as_ref())
                {
                    let dates_hashset = match &captures[1] {
                        "Except" => &mut self.all_dates.except,
                        "Only" => &mut self.all_dates.only,
                        "DG Sailing only" => &mut self.dg_dates.only,
                        other => bail!("Expect \"Except\", \"Only\", or \"DG Sailing only\" in: {:?}", other),
                    };
                    for date_text in captures[3].split(&[',', '&']).map(|s| s.trim()) {
                        let date_within_range = date_range
                            .parse_date_within(
                                date_text,
                                format_description!(
                                    "[month repr:short case_sensitive:false] [day padding:none] [year]"
                                ),
                            )
                            .with_context(|| {
                                format!("Failed to parse date {:?} in {:?}", date_text, annotation_text)
                            })?;
                        if let Some(date) = date_within_range {
                            dates_hashset.insert(date);
                        } else {
                            warn!("Date is outside date range of schedule ({}): {:?}", date_range, date_text);
                        }
                    }
                } else {
                    let replaced_annotation_text = regex!(r"([!#*]*)\s*").replace(annotation_text.as_ref(), "$1 ");
                    let replaced_annotation_text = regex!(r"[\.,]$").replace(replaced_annotation_text.as_ref(), "");
                    let annotation_text = replaced_annotation_text.as_ref().trim();
                    match annotation_text {
                        "! Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing" => {
                            text_date_restriction(
                                &mut self.all_notes,
                                "Saturna-bound vehicles arriving at the booth at least 15 minutes prior to sailing time may be provided loading priority on this sailing"
                            );
                        }
                        "* On Dec 26, Jan 2, Feb 20, the Monday schedule is in effect until 2:00pm, the Holiday Monday Schedule is in effect after 2:00 pm" => {}
                        "Dangerous goods only" |
                        "Dangerous goods sailing" |
                        "DG Sailing only, No other passengers permitted" |
                        "No passengers permitted - DG Sailing only" |
                        "No passengers permitted - only sails on Apr 6, Apr 20, May 4, May 18, Jun 1, Jun 15, Jun 29, Jul 13, Jul 27, Aug 10, Aug 24, Sep 7, Sep 21, Oct 5, Oct 19, Nov 2, Nov 16, Nov 30, Dec 14, Dec 28, Jan 11, Jan 25, Feb 8, Feb 22, Mar 7, Mar 21" |
                        "No passengers permitted - only sails on Jul 2, Jul 16, Jul 30, Aug 13, Aug 27" |
                        "No passengers permitted - only sails on Jul 9, Jul 23, Aug 6, Aug 20, Sep 3" |
                        "No passengers permitted - only sails on May 14, 28, Jun 11 & 25" |
                        "No passengers permitted - only sails on May 14, May 28, Jun 11, Jun 25" |
                        "No passengers permitted - only sails on May 7, May 21, Jun 4, Jun 18" |
                        "No passengers permitted - only sails on Oct 15, Oct 29, Nov 12, Nov 26, Dec 10, Dec 24, Jan 7, Jan 21, Feb 4, Feb 18, Mar 3, Mar 17, Mar 31" |
                        "No passengers permitted - only sails on Oct 22, Nov 5, Nov 19, Dec 3, Dec 17, Dec 31, Jan 14, Jan 28, Feb 11, Feb 25, Mar 10, Mar 24" |
                        "No passengers permitted - only sails on Sep 10, Sep 24, Oct 8" |
                        "No passengers permitted - only sails on Sep 17, Oct 1" => {
                            self.is_dg_only = true;
                        }
                        "Foot passengers only" => {
                            text_date_restriction(&mut self.all_notes, "Foot passengers only");
                        }
                        "No sailings available on this route for these dates" => {}
                        _ => bail!("Unrecognized annotation text: {:?}", annotation_text),
                    }
                }
                Ok(())
            };
            inner().with_context(|| format!("Failed to parse annotation: {:?}", annotation_text.as_ref()))?;
        }
        Ok(())
    }
}
