use crate::annotations::*;
use crate::cache::*;
use crate::constants::*;
use crate::depart_time_and_row_annotations::*;
use crate::imports::*;
use crate::macros::*;
use crate::types::*;
use crate::utils::*;
use crate::weekday_dates::*;

#[derive(Debug)]
struct ScheduleQuery {
    pub date_range: DateRange,
    pub is_route9: bool,
}

impl ScheduleQuery {
    fn parse(schedule_path_query: &str) -> Result<ScheduleQuery> {
        let captures = &regex!("departureDate=([0-9-]*)|departureDateCode=R9_([0-9_]*)")
            .captures(schedule_path_query)
            .ok_or_else(|| {
                anyhow!(
                    "Failed to find departureDate or departureDateCode=R9 in schedule path/query: {:?}",
                    schedule_path_query
                )
            })?;
        let (text, separator, is_route9) = match (captures.get(1), captures.get(2)) {
            (Some(text), _) => (text, "-", false),
            (None, Some(text)) => (text, "_", true),
            (None, None) => panic!("Expect capture to be available when regex matches"),
        };
        Ok(ScheduleQuery {
            date_range: DateRange::parse(text.as_str(), format_description!("[year][month][day]"), separator)
                .with_context(|| format!("Failed to parse schedule path/query date range: {:?}", text))?,
            is_route9,
        })
    }
}

fn parse_annotations(
    depart_times_annotations_texts: Vec<String>,
    date_range: &DateRange,
) -> Result<(Annotations, Vec<String>)> {
    let inner = || {
        let mut depart_times_texts = Vec::new();
        let mut annotations = Annotations::new(date_range);
        for depart_times_annotation_text in depart_times_annotations_texts {
            let trimed_commas = regex!(r"^(,\s*)?(.*)(\s*,)?$").replace(&depart_times_annotation_text, "$2");
            if regex!(r"^\d").is_match(trimed_commas.as_ref()) {
                depart_times_texts.push(depart_times_annotation_text);
            } else {
                match trimed_commas.as_ref() {
                    "" => continue,
                    text => annotations.parse([text], date_range)?,
                }
            }
        }
        Ok((annotations, depart_times_texts)) as Result<_>
    };
    inner().context("Failed to parse depart time/annotations texts")
}

fn parse_depart_times_and_annotations(
    depart_times_texts: Vec<String>,
    annotations: &Annotations,
) -> Result<Vec<DepartTimeAndRowAnnotations>> {
    let inner = || {
        let mut depart_times = Vec::new();
        for depart_times_text in depart_times_texts {
            for depart_time_text in depart_times_text.split(',').filter(|s| !s.is_empty()) {
                depart_times.push(DepartTimeAndRowAnnotations::parse(depart_time_text.trim(), annotations)?)
            }
        }
        Ok(depart_times) as Result<_>
    };
    inner().context("Failed to parse depart times and row annotations")
}

fn parse_duration(duration_text: &str) -> Result<Duration> {
    let inner = || {
        let duration_captures = regex!(r"^((\d+)h)? ?((\d+)m)?$")
            .captures(duration_text)
            .ok_or_else(|| anyhow!("Invalid duration format: {:?}", duration_text))?;
        let duration = match (duration_captures.get(2), duration_captures.get(4)) {
            (None, None) => bail!("Expect minutes and/or hours in duration"),
            (hours_text, minutes_text) => Duration::minutes(
                hours_text.map(|m| m.as_str().parse::<i64>().expect("duration hours to parse to integer")).unwrap_or(0)
                    * 60
                    + minutes_text
                        .map(|m| m.as_str().parse::<i64>().expect("duration minutes to parse to integer"))
                        .unwrap_or(0),
            ),
        };
        Ok(duration) as Result<_>
    };
    inner().with_context(|| format!("Failed to parse duration: {:?}", duration_text))
}

fn parse_stops(stops_texts: Vec<String>) -> Result<Vec<Stop>> {
    let inner = || {
        let stops_chunks = stops_texts.chunks(2);
        let stops = parse_schedule_stops(stops_chunks.map(|items| items.join(" ")))?;
        Ok(stops) as Result<_>
    };
    inner().with_context(|| format!("Failed to parse stops: {:?}", stops_texts))
}

fn parse_route9_table(table_elem: ElementRef, date_range: &DateRange) -> Result<Vec<ScheduleItem>> {
    let inner = || {
        let mut items = Vec::new();
        for day_row_elem in table_elem.select(selector!("thead tr")) {
            let weekday_text = day_row_elem
                .value()
                .attr("data-schedule-day")
                .ok_or_else(|| anyhow!("Expect day row element to have 'data-schedule-day' attribute"))?;
            let weekday_sailings_tbody_elem = day_row_elem
                .parent_element()
                .expect("weekday row element to have parent")
                .next_sibling_element()
                .ok_or_else(|| anyhow!("Expect schedule row thead element after weekday row element"))?;
            for sailing_row_elem in weekday_sailings_tbody_elem.select(selector!("tr.schedule-table-row")) {
                let cell_elems: Vec<_> = sailing_row_elem.select(selector!("td")).collect();
                ensure!(
                    cell_elems.len() == 6,
                    "Row should have six cells: {:?}",
                    cell_elems.iter().map(element_text).collect::<Vec<_>>()
                );
                let (annotations, depart_times_texts) = parse_annotations(element_texts(&cell_elems[1]), date_range)?;
                let depart_times = parse_depart_times_and_annotations(depart_times_texts, &annotations)?;
                ensure!(depart_times.len() == 1, "Expect exactly one depart time in row");
                let depart_time = depart_times.into_iter().next().expect("at least one depart time in row");
                let weekday_dates = WeekdayDates::parse(weekday_text, &annotations, date_range)?;
                let arrive_time = parse_schedule_time(&element_text(&cell_elems[2]))?;
                let stops = parse_stops(element_texts(&cell_elems[4]))?;
                let weekdays = weekday_dates.to_date_restrictions(&depart_time.row_dates);
                let notes = AnnotationDates::map_to_date_restrictions_by_weekdays(depart_time.row_notes, &weekdays);
                items.push(ScheduleItem {
                    sailing: Sailing { depart_time: depart_time.time, arrive_time, stops: stops.clone() },
                    weekdays,
                    notes,
                });
            }
        }
        ScheduleItem::merge_items(items)
    };
    inner().context("Failed to parse other route schedule table")
}

fn parse_non_route9_table(table_elem: ElementRef, date_range: &DateRange) -> Result<Vec<ScheduleItem>> {
    let inner = || {
        let mut last_row_weekday_dates = WeekdayDates::new();
        let mut items = Vec::new();
        for row_elem in table_elem.select(selector!("tr.schedule-table-row")) {
            let cell_elems: Vec<_> = row_elem.select(selector!("td")).collect();
            if cell_elems.is_empty() {
                continue;
            };
            if cell_elems.len() == 2 && element_text(&cell_elems[1]) == "LEGEND Non-stop Transfer Stop" {
                break;
            }
            ensure!(
                cell_elems.len() == 6,
                "Row should have six cells: {:?}",
                cell_elems.iter().map(element_text).collect::<Vec<_>>()
            );
            let (annotations, depart_times_texts) = parse_annotations(element_texts(&cell_elems[2]), date_range)?;
            let depart_times = parse_depart_times_and_annotations(depart_times_texts, &annotations)?;
            let day_text = element_text(&cell_elems[1]);
            let weekday_dates = if day_text.is_empty() {
                last_row_weekday_dates
            } else {
                WeekdayDates::parse(&day_text, &annotations, date_range)?
            };
            let duration = parse_duration(&element_text(&cell_elems[3]))?;
            let stops = parse_stops(element_texts(&cell_elems[4]))?;
            for depart_time in depart_times {
                let weekdays = weekday_dates.to_date_restrictions(&depart_time.row_dates);
                let notes = AnnotationDates::map_to_date_restrictions_by_weekdays(depart_time.row_notes, &weekdays);
                items.push(ScheduleItem {
                    sailing: Sailing {
                        depart_time: depart_time.time,
                        arrive_time: depart_time.time + duration,
                        stops: stops.clone(),
                    },
                    weekdays,
                    notes,
                });
            }
            last_row_weekday_dates = weekday_dates;
        }
        ScheduleItem::merge_items(items)
    };
    inner().context("Failed to parse other route schedule table")
}

async fn scrape_schedule(
    options: &Options,
    cache: &Cache<'_>,
    terminal_pair: TerminalPair,
    schedule_path_query_text: &str,
    today: Date,
) -> Result<Option<Schedule>> {
    let source_url = format!("{}{}", BCFERRIES_BASE_URL, schedule_path_query_text);
    let inner = async {
        let ScheduleQuery { date_range, is_route9 } = ScheduleQuery::parse(schedule_path_query_text)
            .with_context(|| format!("Failed to schedule path/query: {:?}", schedule_path_query_text))?;
        if !should_scrape_schedule_date(date_range, today, options.date) {
            return Ok(None);
        }
        let document = cache
            .get_html(&source_url, &HTML_ERROR_REGEX)
            .await
            .with_context(|| format!("Failed to download schedule HTML from: {:?}", source_url))?;
        info!("Parsing schedule for {}, {}", terminal_pair, date_range);
        let table_elem = document
            .select(selector!("div.seasonal-schedule-wrapper table"))
            .next()
            .ok_or_else(|| anyhow!("Missing table element in schedule"))?;
        let items = if is_route9 {
            parse_route9_table(table_elem, &date_range)?
        } else {
            parse_non_route9_table(table_elem, &date_range)?
        };
        Ok(Some(Schedule {
            terminal_pair,
            date_range,
            items,
            source_url: source_url.to_string(),
            refreshed_at: now_vancouver(),
            alerts: vec![],
        })) as Result<_>
    };
    inner
        .await
        .with_context(|| format!("Failed to scrape other route schedule for {} from: {:?}", terminal_pair, source_url))
}

pub async fn scrape_other_route_schedules(
    options: &Options,
    cache: &Cache<'_>,
    terminal_pair: TerminalPair,
    today: Date,
) -> Result<Vec<Schedule>> {
    if options.terminals.is_some() && options.terminals != Some(terminal_pair) {
        return Ok(vec![]);
    }
    let base_url = format!("{}/{}", OTHER_ROUTE_SCHEDULES_BASE_URL, terminal_pair.to_schedule_code_pair());
    let inner = async {
        let base_document = cache
            .get_html(&base_url, &HTML_ERROR_REGEX)
            .await
            .with_context(|| format!("Failed to download base schedule HTML from: {:?}", base_url))?;
        let schedule_path_query_elems = base_document.select(selector!("div#dateRangeModal a"));
        let mut schedules = Vec::new();
        for schedule_path_query_elem in schedule_path_query_elems.take(1) {
            let schedule_path_query = schedule_path_query_elem.value().attr("href").ok_or_else(|| {
                anyhow!("Missing schedule path/query in date range link element: {}", schedule_path_query_elem.html())
            })?;
            let opt_schedule = scrape_schedule(options, cache, terminal_pair, schedule_path_query, today).await?;
            opt_schedule.iter().for_each(|s| debug!("Parsed schedule: {:#?}", s));
            schedules.extend(opt_schedule);
        }
        ensure!(!schedules.is_empty(), "Failed to find any schedule elements");
        Ok(schedules) as Result<_>
    };
    inner
        .await
        .with_context(|| format!("Failed to scrape other route schedule for {} from: {:?}", terminal_pair, base_url))
}
