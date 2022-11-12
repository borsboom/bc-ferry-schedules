use ferrysched_shared::constants::*;
use ferrysched_shared::imports::*;
use ferrysched_shared::types::*;

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct SailingWithNotes {
    pub sailing: Sailing,
    pub notes: Vec<String>,
}

fn schedule_sailings_for_date(schedule: &Schedule, date: Date) -> Vec<SailingWithNotes> {
    let mut sailings = Vec::new();
    for item in &schedule.items {
        if let Some(weekday_dr) = item.weekdays.get(&date.weekday()) {
            if weekday_dr.includes_date(date) {
                let notes = item
                    .notes
                    .iter()
                    .filter_map(|(a, dr)| dr.includes_date(date).then(|| a.as_ref()))
                    .map(String::from)
                    .collect();
                sailings.push(SailingWithNotes { sailing: item.sailing.clone(), notes });
            }
        }
    }
    sailings
}

fn schedules_sailings_for_date(schedules: &[Schedule], date: Date) -> Option<(&Schedule, Vec<SailingWithNotes>)> {
    schedules
        .iter()
        .filter(|sched| sched.date_range.includes_date_inclusive(date))
        .map(|sched| (sched, schedule_sailings_for_date(sched, date)))
        .next()
}

fn terminal_pair_sailings_for_date(
    terminal_pair: TerminalPair,
    date: Date,
    schedules_map: &HashMap<TerminalPair, Vec<Schedule>>,
) -> Option<(&Schedule, Vec<SailingWithNotes>)> {
    if let Some((schedule, mut sailings)) =
        schedules_map.get(&terminal_pair).and_then(|schedules| schedules_sailings_for_date(schedules, date))
    {
        sailings.sort_unstable();
        Some((schedule, sailings))
    } else {
        None
    }
}

pub fn area_sailings_for_date(
    area_pair: AreaPair,
    date: Date,
    schedules_map: &HashMap<TerminalPair, Vec<Schedule>>,
) -> Option<Vec<(&Schedule, Vec<SailingWithNotes>)>> {
    let mut area_schedules_vec = AREA_PAIR_TERMINAL_PAIRS
        .get(&area_pair)
        .map(|tps| tps.iter().filter_map(|&tp| terminal_pair_sailings_for_date(tp, date, schedules_map)).collect())
        .unwrap_or_else(Vec::new);
    (!area_schedules_vec.is_empty()).then(|| {
        area_schedules_vec.sort_unstable_by(|(sa, va), (sb, vb)| {
            va.len().cmp(&vb.len()).reverse().then_with(|| sa.terminal_pair.cmp(&sb.terminal_pair))
        });
        area_schedules_vec.into_iter().filter(|(s, v)| !v.is_empty() || !s.alerts.is_empty()).collect()
    })
}
