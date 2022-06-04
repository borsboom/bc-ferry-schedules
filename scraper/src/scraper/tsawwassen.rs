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

fn parse_duration_and_stops(duration_stops_texts: Vec<String>) -> Result<(Duration, Vec<Stop>)> {
    let inner = || {
        ensure!(duration_stops_texts.len() >= 2, "Duration and stops should have at least two values");
        let duration_text = &duration_stops_texts[0];
        let duration_captures = regex!(r"^(\d+)h (\d+)m$")
            .captures(duration_text)
            .ok_or_else(|| anyhow!("Invalid duration format: {:?}", duration_text))?;
        let duration = Duration::minutes(
            duration_captures[1].parse::<i64>().unwrap() * 60 + duration_captures[2].parse::<i64>().unwrap(),
        );
        let stops_text = &duration_stops_texts[1..];
        let stops =
            parse_schedule_stops(stops_text).with_context(|| format!("Failed to parse stops: {:?}", stops_text))?;
        Ok((duration, stops))
    };
    inner().with_context(|| format!("Failed to parse duration and stops: {:?}", duration_stops_texts))
}

fn parse_table(table_elem: ElementRef, date_range: &DateRange) -> Result<Vec<ScheduleItem>> {
    let inner = || {
        let mut last_row_weekday_dates = WeekdayDates::new();
        let mut items = Vec::new();
        for row_elem in table_elem.select(selector!("tr")) {
            let cell_elems: Vec<_> = row_elem.select(selector!("td")).collect();
            if cell_elems.is_empty() {
                continue;
            };
            if cell_elems.len() == 2 && element_text(&cell_elems[1]) == "LEGEND Non-stop Transfer Stop" {
                break;
            }
            ensure!(
                cell_elems.len() == 5,
                "Row should have five cells: {:?}",
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
            let (duration, stops) = parse_duration_and_stops(element_texts(&cell_elems[3]))?;
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

fn parse_date_range(text: &str) -> Result<DateRange> {
    DateRange::parse(text, format_description!("[year][month][day]"), "-")
        .with_context(|| format!("Failed to parse schedule query date range: {:?}", text))
}

fn schedule_base_url(terminal_pair: TerminalCodePair) -> String {
    format!("https://www.bcferries.com/routes-fares/schedules/seasonal/{}", terminal_pair.to_schedule_code_pair())
}

async fn scrape_schedule(
    options: &Options,
    cache: &Cache<'_>,
    terminal_pair: TerminalCodePair,
    date_range_query_value: &str,
    today: Date,
) -> Result<Option<Schedule>> {
    let source_url = format!("{}?departureDate={}", schedule_base_url(terminal_pair), date_range_query_value);
    let inner = async {
        let date_range = parse_date_range(date_range_query_value)
            .with_context(|| format!("Failed to parse date range query value: {:?}", date_range_query_value))?;
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
    inner.await.with_context(|| {
        format!(
            "Failed to scrape Tsawwassen schedule for {}, {} from: {:?}",
            terminal_pair, date_range_query_value, source_url
        )
    })
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
    let base_url = schedule_base_url(terminal_pair);
    let inner = async {
        let base_document = cache
            .get_html(&base_url, &HTML_ERROR_REGEX)
            .await
            .with_context(|| format!("Failed to download base schedule HTML from: {:?}", base_url))?;
        let schedule_container_elem = base_document
            .select(selector!("div.seasonalSchedulesContainer"))
            .next()
            .ok_or_else(|| anyhow!("Missing schedule container element"))?;
        let date_range_option_elems = schedule_container_elem.select(selector!("select#seasonalScheduleDate option"));
        let mut schedules = Vec::new();
        for date_range_option_elem in date_range_option_elems {
            let date_range_option_value = date_range_option_elem.value().attr("value").ok_or_else(|| {
                anyhow!("Missing value in date range option element: {}", date_range_option_elem.html())
            })?;
            let opt_schedule = scrape_schedule(options, cache, terminal_pair, date_range_option_value, today).await?;
            opt_schedule.iter().for_each(|s| debug!("Parsed schedule: {:#?}", s));
            schedules.extend(opt_schedule);
        }
        Ok(schedules) as Result<_>
    };
    inner
        .await
        .with_context(|| format!("Failed to scrape Tsawwassen schedule for {} from: {:?}", terminal_pair, base_url))
}
