use crate::annotations::*;
use crate::cache::*;
use crate::constants::*;
use crate::depart_time_and_row_annotations::*;
use crate::imports::*;
use crate::macros::*;
use crate::types::*;
use crate::utils::*;
use crate::weekday_dates::*;

fn parse_annotations(
    depart_times_annotations_texts: Vec<String>,
    date_range: &DateRange,
) -> Result<(Annotations, Vec<String>)> {
    let inner = || {
        let mut depart_times_texts = Vec::new();
        let mut annotations = Annotations::new();
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
        let duration_captures = regex!(r"^(\d+)h (\d+)m$")
            .captures(duration_text)
            .ok_or_else(|| anyhow!("Invalid duration format: {:?}", duration_text))?;
        let duration = Duration::minutes(
            duration_captures[1].parse::<i64>().expect("duration hours to parse to integer") * 60
                + duration_captures[2].parse::<i64>().expect("duration minutes to parse to integer"),
        );
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

fn parse_table(table_elem: ElementRef, date_range: &DateRange) -> Result<Vec<ScheduleItem>> {
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
    inner().context("Failed to parse Tsawwassen schedule table")
}

fn parse_date_range_from_schedule_query(schedule_query: &str) -> Result<DateRange> {
    let text = &regex!("departureDate=([0-9-]*)")
        .captures(schedule_query)
        .ok_or_else(|| anyhow!("Failed to find departureDate in schedule query: {:?}", schedule_query))?[1];
    DateRange::parse(text, format_description!("[year][month][day]"), "-")
        .with_context(|| format!("Failed to parse schedule query date range: {:?}", text))
}

async fn scrape_schedule(
    options: &Options,
    cache: &Cache<'_>,
    terminal_pair: TerminalCodePair,
    schedule_query: &str,
    today: Date,
) -> Result<Option<Schedule>> {
    let source_url = format!("https://www.bcferries.com/{}", schedule_query);
    let inner = async {
        let date_range = parse_date_range_from_schedule_query(schedule_query)
            .with_context(|| format!("Failed to parse date from schedule query: {:?}", schedule_query))?;
        if !should_scrape_schedule_date(date_range, today, options.date) {
            return Ok(None);
        }
        let document = cache
            .get_html(&source_url, &HTML_ERROR_REGEX)
            .await
            .with_context(|| format!("Failed to download schedule HTML from: {:?}", source_url))?;
        info!("Parsing schedule for {}, {}", terminal_pair, date_range);
        let table_elem = document
            .select(selector!("div.seasonalSchedulesContainer table"))
            .next()
            .ok_or_else(|| anyhow!("Missing table element in schedule"))?;
        let items = parse_table(table_elem, &date_range)?;
        Ok(Some(Schedule {
            terminal_pair,
            date_range,
            items,
            source_url: source_url.to_string(),
            refreshed_at: now_vancouver(),
        })) as Result<_>
    };
    inner
        .await
        .with_context(|| format!("Failed to scrape Tsawwassen schedule for {} from: {:?}", terminal_pair, source_url))
}

pub async fn scrape_tsawwassen_schedules(
    options: &Options,
    cache: &Cache<'_>,
    terminal_pair: TerminalCodePair,
    today: Date,
) -> Result<Vec<Schedule>> {
    if options.terminals.is_some() && options.terminals != Some(terminal_pair) {
        return Ok(vec![]);
    }
    let base_url =
        format!("https://www.bcferries.com/routes-fares/schedules/seasonal/{}", terminal_pair.to_schedule_code_pair());
    let inner = async {
        let base_document = cache
            .get_html(&base_url, &HTML_ERROR_REGEX)
            .await
            .with_context(|| format!("Failed to download base schedule HTML from: {:?}", base_url))?;
        let schedule_query_elems = base_document.select(selector!("div#dateRangeModal a"));
        let mut schedules = Vec::new();
        for schedule_query_elem in schedule_query_elems {
            let schedule_query = schedule_query_elem.value().attr("href").ok_or_else(|| {
                anyhow!("Missing schedule query in date range link element: {}", schedule_query_elem.html())
            })?;
            let opt_schedule = scrape_schedule(options, cache, terminal_pair, schedule_query, today).await?;
            opt_schedule.iter().for_each(|s| debug!("Parsed schedule: {:#?}", s));
            schedules.extend(opt_schedule);
        }
        ensure!(!schedules.is_empty(), "Failed to find any schedule elements");
        Ok(schedules) as Result<_>
    };
    inner
        .await
        .with_context(|| format!("Failed to scrape Tsawwassen schedule for {} from: {:?}", terminal_pair, base_url))
}
