use crate::imports::*;
use crate::types::*;

#[derive(Debug)]
pub struct SailingWithNotes {
    pub sailing: Sailing,
    pub notes: Vec<&'static str>,
}

#[derive(Debug)]
pub struct Sailings {
    pub terminal_pair: TerminalCodePair,
    pub date: NaiveDate,
    pub sailings: Vec<SailingWithNotes>,
}

impl Sailings {
    pub fn from_schedule(options: &Options, schedule: &Schedule) -> Vec<Sailings> {
        let mut result = Vec::new();
        let dates_iter: Box<dyn Iterator<Item = NaiveDate>> = match options.date {
            Some(_) => Box::new(options.date.iter().copied()),
            _ => Box::new(schedule.effective_date_range.iter_days()),
        };
        for date in dates_iter {
            let mut sailings = Vec::new();
            for item in &schedule.items {
                if let Some(weekday_dr) = item.weekdays.get(&date.weekday()) {
                    if weekday_dr.includes_date(date) {
                        let notes = item
                            .notes
                            .iter()
                            .filter_map(|(a, dr)| if dr.includes_date(date) { Some(a) } else { None })
                            .copied()
                            .collect();
                        sailings.push(SailingWithNotes { sailing: item.sailing.clone(), notes });
                    }
                }
            }
            result.push(Sailings { terminal_pair: schedule.terminal_pair, date, sailings });
        }
        debug!("Sailings: {:#?}", result);
        result
    }
}
