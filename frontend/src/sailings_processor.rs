use ferrysched_shared::imports::*;
use ferrysched_shared::types::*;

static MIN_THRU_FARE_TRANSFER_DURATION: Lazy<Duration> = Lazy::new(|| Duration::minutes(30));
static MAX_THRU_FARE_TRANSFER_DURATION: Lazy<Duration> = Lazy::new(|| Duration::minutes(125));

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct SailingWithNotes {
    pub sailing: Sailing,
    pub notes: Vec<String>,
}

fn schedule_sailings_for_date(schedule: &Schedule, date: NaiveDate) -> Vec<SailingWithNotes> {
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

fn schedules_sailings_for_date(schedules: &[Schedule], date: NaiveDate) -> Option<(&Schedule, Vec<SailingWithNotes>)> {
    schedules
        .iter()
        .filter(|sched| sched.date_range.includes_date_inclusive(date))
        .map(|sched| (sched, schedule_sailings_for_date(sched, date)))
        .next()
}

fn get_potential_thrufare_sailings(
    to_swb_sailings: Vec<SailingWithNotes>,
    from_swb_sailings: Vec<SailingWithNotes>,
) -> Vec<SailingWithNotes> {
    let mut thrufare_sailings = Vec::new();
    for from_swb in &from_swb_sailings {
        let swb_arrive_time_range = (from_swb.sailing.depart_time - *MAX_THRU_FARE_TRANSFER_DURATION)
            ..=(from_swb.sailing.depart_time - *MIN_THRU_FARE_TRANSFER_DURATION);
        for to_swb in
            to_swb_sailings.iter().filter(|to_swb| swb_arrive_time_range.contains(&to_swb.sailing.arrive_time))
        {
            let mut stops = to_swb.sailing.stops.clone();
            stops.push(Stop { type_: StopType::Thrufare, terminal: TerminalCode::SWB });
            stops.extend(&from_swb.sailing.stops);
            let mut notes = vec!["Connection at Victoria not guaranteed".to_string()];
            notes.extend(to_swb.notes.iter().map(|note| format!("To Victoria: {}", note)));
            notes.extend(from_swb.notes.iter().map(|note| format!("From Victoria: {}", note)));
            thrufare_sailings.push(SailingWithNotes {
                sailing: Sailing {
                    depart_time: to_swb.sailing.depart_time,
                    arrive_time: from_swb.sailing.arrive_time,
                    stops,
                },
                notes,
            })
        }
    }
    thrufare_sailings
}

fn select_thrufare_sailings(
    terminal_pair: TerminalCodePair,
    mut thrufare_sailings: Vec<SailingWithNotes>,
) -> Vec<SailingWithNotes> {
    if terminal_pair.to == TerminalCode::TSA {
        for depart_time in thrufare_sailings.iter().map(|s| s.sailing.depart_time).collect::<HashSet<_>>() {
            if let Some(max_arrive_time) = thrufare_sailings
                .iter()
                .filter(|s| s.sailing.depart_time == depart_time)
                .map(|s| s.sailing.arrive_time)
                .max()
            {
                thrufare_sailings
                    .retain(|s| s.sailing.depart_time != depart_time || s.sailing.arrive_time == max_arrive_time);
            }
        }
    } else {
        for arrive_time in thrufare_sailings.iter().map(|s| s.sailing.arrive_time).collect::<HashSet<_>>() {
            if let Some(min_depart_time) = thrufare_sailings
                .iter()
                .filter(|s| s.sailing.arrive_time == arrive_time)
                .map(|s| s.sailing.depart_time)
                .min()
            {
                thrufare_sailings
                    .retain(|s| s.sailing.arrive_time != arrive_time || s.sailing.depart_time == min_depart_time);
            }
        }
    }
    thrufare_sailings
}

fn get_thrufare_sailings(
    terminal_pair: TerminalCodePair,
    date: NaiveDate,
    schedules_map: &HashMap<TerminalCodePair, Vec<Schedule>>,
) -> Vec<SailingWithNotes> {
    if let (Some((_, to_swb_sailings)), Some((_, from_swb_sailings))) = (
        schedules_map
            .get(&TerminalCodePair { from: terminal_pair.from, to: TerminalCode::SWB })
            .and_then(|schedules| schedules_sailings_for_date(schedules, date)),
        schedules_map
            .get(&TerminalCodePair { from: TerminalCode::SWB, to: terminal_pair.to })
            .and_then(|schedules| schedules_sailings_for_date(schedules, date)),
    ) {
        select_thrufare_sailings(terminal_pair, get_potential_thrufare_sailings(to_swb_sailings, from_swb_sailings))
    } else {
        vec![]
    }
}

pub fn sailings_for_date(
    terminal_pair: TerminalCodePair,
    date: NaiveDate,
    schedules_map: &HashMap<TerminalCodePair, Vec<Schedule>>,
) -> Option<(&Schedule, Vec<SailingWithNotes>)> {
    if let Some((schedule, mut sailings)) =
        schedules_map.get(&terminal_pair).and_then(|schedules| schedules_sailings_for_date(schedules, date))
    {
        if (terminal_pair.from == TerminalCode::TSA && terminal_pair.to != TerminalCode::SWB)
            || (terminal_pair.to == TerminalCode::TSA && terminal_pair.from != TerminalCode::SWB)
        {
            sailings.extend(get_thrufare_sailings(terminal_pair, date, schedules_map));
        }
        sailings.sort_unstable();
        Some((schedule, sailings))
    } else {
        None
    }
}
