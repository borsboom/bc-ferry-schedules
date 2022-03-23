use crate::annotations::*;
use crate::imports::*;
use crate::macros::*;
use crate::utils::*;

pub struct DepartTimeAndRowAnnotations {
    pub time: NaiveTime,
    pub row_dates: AnnotationDates,
    pub row_notes: HashMap<Cow<'static, str>, AnnotationDates>,
}

impl DepartTimeAndRowAnnotations {
    pub fn parse(orig_text: &str, annotations: &Annotations) -> Result<DepartTimeAndRowAnnotations> {
        let star_suffix_re: &Regex = regex!(r"(M) ?\*$");
        let star2_suffix_re: &Regex = regex!(r"(M) ?\*\*$");
        let exclamation_suffix_re: &Regex = regex!(r"(M) ?!$");
        let hash_suffix_re: &Regex = regex!(r"(M) ?#$");
        let plus_suffix_re: &Regex = regex!(r"(M) ?\+$");
        let exclamation2_suffix_re: &Regex = regex!(r"(M) ?!!$");
        let exclamation_plus_suffix_re: &Regex = regex!(r"(M) ?! ?\+$");
        let exclamation_hash_suffix_re: &Regex = regex!(r"(M) ?! ?#$");
        let mut row_dates = AnnotationDates::new();
        let mut row_dates_by_time = HashMap::new();
        let mut row_notes = HashMap::new();
        let text = if exclamation_plus_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.exclamation);
            row_dates.extend(&annotations.plus);
            row_notes.extend(annotations.exclamation_text.clone().into_iter());
            row_notes.extend(annotations.plus_text.clone().into_iter());
            exclamation_plus_suffix_re.replace(orig_text, "$1")
        } else if exclamation_hash_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.exclamation);
            row_dates.extend(&annotations.hash);
            row_notes.extend(annotations.exclamation_text.clone().into_iter());
            row_notes.extend(annotations.hash_text.clone().into_iter());
            exclamation_hash_suffix_re.replace(orig_text, "$1")
        } else if exclamation2_suffix_re.is_match(orig_text) {
            row_notes.extend(annotations.exclamation2_text.clone().into_iter());
            exclamation2_suffix_re.replace(orig_text, "$1")
        } else if star2_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.star2);
            star2_suffix_re.replace(orig_text, "$1")
        } else if star_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.star);
            row_dates_by_time.extend(&annotations.star_by_time);
            star_suffix_re.replace(orig_text, "$1")
        } else if exclamation_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.exclamation);
            row_notes.extend(annotations.exclamation_text.clone().into_iter());
            exclamation_suffix_re.replace(orig_text, "$1")
        } else if hash_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.hash);
            row_notes.extend(annotations.hash_text.clone().into_iter());
            hash_suffix_re.replace(orig_text, "$1")
        } else if plus_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.plus);
            row_notes.extend(annotations.plus_text.clone().into_iter());
            plus_suffix_re.replace(orig_text, "$1")
        } else {
            Cow::from(orig_text)
        };
        let depart_time = parse_schedule_time(&text)
            .with_context(|| format!("Invalid depart time in {:?}: {:?}", orig_text, text))?;
        if let Some(time_date_restriction) = row_dates_by_time.get(&depart_time) {
            row_dates.extend(time_date_restriction);
        }
        Ok(DepartTimeAndRowAnnotations { time: depart_time, row_dates, row_notes })
    }
}
