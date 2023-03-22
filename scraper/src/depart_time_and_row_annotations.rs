use crate::annotations::*;
use crate::imports::*;
use crate::macros::*;
use crate::utils::*;

#[derive(Debug)]
pub struct DepartTimeAndRowAnnotations {
    pub time: Time,
    pub row_dates: AnnotationDates,
    pub row_notes: AnnotationNotes,
}

impl DepartTimeAndRowAnnotations {
    pub fn parse(orig_text: &str, annotations: &Annotations) -> Result<DepartTimeAndRowAnnotations> {
        let star_suffix_re: &Regex = regex!(r"(?i)(M) ?\*$");
        let star2_suffix_re: &Regex = regex!(r"(?i)(M) ?\*\*$");
        let exclamation_suffix_re: &Regex = regex!(r"(?i)(M) ?!$");
        let hash_suffix_re: &Regex = regex!(r"(?i)(M) ?#$");
        let hash_prefix_re: &Regex = regex!(r"(?i)^# ?([0-9])");
        let plus_suffix_re: &Regex = regex!(r"(?i)(M) ?\+$");
        let exclamation2_suffix_re: &Regex = regex!(r"(?i)(M) ?!!$");
        let exclamation_plus_suffix_re: &Regex = regex!(r"(?i)(M) ?! ?\+$");
        let exclamation_hash_suffix_re: &Regex = regex!(r"(?i)(M) ?! ?#$");
        let mut row_dates = AnnotationDates::new();
        let mut row_dates_by_time = HashMap::new();
        let mut row_notes = AnnotationNotes::new();
        let text = if exclamation_plus_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.exclamation_dates);
            row_dates.extend(&annotations.plus_dates);
            row_notes.extend(annotations.exclamation_notes.clone());
            row_notes.extend(annotations.plus_notes.clone());
            exclamation_plus_suffix_re.replace(orig_text, "$1")
        } else if exclamation_hash_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.exclamation_dates);
            row_dates.extend(&annotations.hash_dates);
            row_notes.extend(annotations.exclamation_notes.clone());
            row_notes.extend(annotations.hash_notes.clone());
            exclamation_hash_suffix_re.replace(orig_text, "$1")
        } else if exclamation2_suffix_re.is_match(orig_text) {
            row_notes.extend(annotations.exclamation2_notes.clone());
            exclamation2_suffix_re.replace(orig_text, "$1")
        } else if star2_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.star2_dates);
            star2_suffix_re.replace(orig_text, "$1")
        } else if star_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.star_dates);
            row_dates_by_time.extend(&annotations.star_dates_by_time);
            star_suffix_re.replace(orig_text, "$1")
        } else if exclamation_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.exclamation_dates);
            row_notes.extend(annotations.exclamation_notes.clone());
            exclamation_suffix_re.replace(orig_text, "$1")
        } else if hash_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.hash_dates);
            row_notes.extend(annotations.hash_notes.clone());
            hash_suffix_re.replace(orig_text, "$1")
        } else if hash_prefix_re.is_match(orig_text) {
            row_dates.extend(&annotations.hash_dates);
            row_notes.extend(annotations.hash_notes.clone());
            hash_prefix_re.replace(orig_text, "$1")
        } else if plus_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.plus_dates);
            row_notes.extend(annotations.plus_notes.clone());
            plus_suffix_re.replace(orig_text, "$1")
        } else {
            row_dates.extend(&annotations.all_dates);
            row_notes.extend(annotations.all_notes.clone());
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
