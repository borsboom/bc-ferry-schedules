use crate::imports::*;
use crate::types::*;

#[derive(Debug)]
pub struct Sailings {
    pub terminal_pair: TerminalPair,
    pub date: NaiveDate,
    pub sailings: Vec<Sailing>,
}

impl Sailings {
    pub fn from_schedule(schedule: &Schedule) -> Result<Vec<Sailings>> {
        let mut result = Vec::new();
        for date in schedule.effective_date_range.iter_days() {
            let mut sailings = Vec::new();
            for item in &schedule.items {
                if !item.except_dates.contains(&date) {
                    if let Some(weekday) = item.weekdays.get(&date.weekday()) {
                        if weekday.only_dates.is_empty() || weekday.only_dates.contains(&date) {
                            sailings.push(item.sailing.clone());
                        }
                    }
                }
            }
            result.push(Sailings { terminal_pair: schedule.terminal_pair, date, sailings });
        }
        Ok(result)
    }
}
