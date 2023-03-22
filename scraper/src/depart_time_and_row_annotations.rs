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
        let mut row_dates = AnnotationDates::new();
        let mut row_dates_by_time = HashMap::new();
        let mut row_notes = AnnotationNotes::new();
        let text = if star_suffix_re.is_match(orig_text) {
            row_dates.extend(&annotations.star_dates);
            row_dates_by_time.extend(&annotations.star_dates_by_time);
            star_suffix_re.replace(orig_text, "$1")
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
