use crate::annotations::*;
use crate::cache::*;
use crate::constants::*;
use crate::depart_time_and_row_annotations::*;
use crate::imports::*;
use crate::macros::*;
use crate::types::*;
use crate::utils::*;

fn parse_annotations(
    depart_times_annotations_texts: Vec<String>,
    date_range: &DateRange,
) -> Result<Option<(Annotations, Vec<String>)>> {
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
                    text => annotations.parse(date_range, [text])?,
                }
            }
        }
        if annotations.is_dg_only {
            Ok(None) as Result<_>
        } else {
            ensure!(annotations.dg_dates.is_always(), "Expect no dangeous goods dates for non-DG sailing");
            Ok(Some((annotations, depart_times_texts))) as Result<_>
        }
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
        let mut items = Vec::new();
        for day_row_elem in table_elem.select(selector!("thead tr")) {
            let weekday_text = day_row_elem
                .value()
                .attr("data-schedule-day")
                .ok_or_else(|| anyhow!("Expect day row element to have 'data-schedule-day' attribute"))?;
            let weekday_sailings_tbody_elem = day_row_elem
                .parent_element()
                .expect("Expect weekday row element to have parent")
                .next_sibling_element()
                .ok_or_else(|| anyhow!("Expect schedule row thead element after weekday row element"))?;
            for sailing_row_elem in weekday_sailings_tbody_elem.select(selector!("tr.schedule-table-row")) {
                let cell_elems: Vec<_> = sailing_row_elem.select(selector!("td")).collect();
                ensure!(
                    cell_elems.len() == 6,
                    "Row should have six cells: {:?}",
                    cell_elems.iter().map(element_text).collect::<Vec<_>>()
                );
                let (annotations, depart_times_texts) =
                    match parse_annotations(element_texts(&cell_elems[1]), date_range)? {
                        None => continue,
                        Some(result) => result,
                    };
                let depart_times = parse_depart_times_and_annotations(depart_times_texts, &annotations)?;
                ensure!(depart_times.len() == 1, "Expect exactly one depart time in row");
                let depart_time = depart_times.into_iter().next().expect("Expect at least one depart time in row");
                let weekday = parse_weekday(weekday_text)?;
                let arrive_time = parse_arrive_time_or_duration(depart_time.time, &element_text(&cell_elems[2]))?;
                if arrive_time != depart_time.time {
                    let stops = parse_stops(element_texts(&cell_elems[4]))?;
                    let date_restriction = depart_time.row_dates.into_date_restriction_by_weekday(weekday);
                    let notes = annotation_notes_date_restictions(depart_time.row_notes, weekday, &date_restriction);
                    items.push(ScheduleItem {
                        sailing: Sailing { depart_time: depart_time.time, arrive_time, stops: stops.clone() },
                        weekdays: HashMap::from_iter([(weekday, date_restriction)]),
                        notes,
                    });
                }
            }
        }
        ScheduleItem::merge_items(items)
    };
    inner().context("Failed to parse route schedule table")
}

async fn scrape_schedule(
    options: &Options,
    source_url: &str,
    document: &Html,
    terminal_pair: TerminalPair,
    index: usize,
    today: Date,
) -> Result<Option<Schedule>> {
    let inner = async {
        let date_range_text = element_text(
            &document
                .select(selector!("div.schedule-custom-calendar a"))
                .next()
                .context("Missing schedule calendar header link element")?,
        );
        let date_range = DateRange::parse(
            &date_range_text,
            format_description!("[month repr:short case_sensitive:false] [day], [year]"),
            " - ",
        )
        .with_context(|| format!("Failed to parse date range: {:?}", date_range_text))?;
        if !should_scrape_schedule_date(date_range, today, options.date) {
            return Ok(None);
        }
        if DISABLED_TERMINAL_PAIRS.contains(&terminal_pair) {
            info!("Skipping parsing disabled schedule for {}, {}", terminal_pair, date_range);
            return Ok(Some(Schedule {
                terminal_pair,
                date_range,
                items: vec![],
                source_url: source_url.to_string(),
                refreshed_at: now_vancouver(),
                alerts: vec![Alert {message: "THIS SCHEDULE IS CURRENTLY UNAVAILABLE!  BC Ferries has update the schedule format on their website and the scraper needs to be updated to understand it.".to_string(), level: AlertLevel::Danger}],
            }));
        }
        info!("Parsing schedule for {}, {}", terminal_pair, date_range);
        let opt_table_elem = document.select(selector!("div.seasonal-schedule-wrapper table")).next();
        if let Some(table_elem) = opt_table_elem {
            let items = parse_table(table_elem, &date_range)?;
            Ok(Some(Schedule {
                terminal_pair,
                date_range,
                items,
                source_url: source_url.to_string(),
                refreshed_at: now_vancouver(),
                alerts: vec![],
            })) as Result<_>
        } else if index == 0 {
            for elem in document.select(selector!("div.seasonalSchedulesContainer div.text-center")) {
                if element_text(&elem).contains("Seasonal schedules have not been posted for these dates") {
                    return Ok(None);
                }
            }
            // If the table element is missing in the initial schedule page for the route, we have a problem
            bail!("Missing table element in schedule");
        } else {
            // However, sometimes the initial page has links to non-existent schedules in which case we can ignore them
            Ok(None)
        }
    };
    inner.await.with_context(|| format!("Failed to scrape route schedule for {} from: {:?}", terminal_pair, source_url))
}

pub async fn scrape_route_schedules(
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
        let date_range_modal_elem =
            base_document.select(selector!("div#dateRangeModal")).next().context("Missing date range modal")?;
        let schedule_path_query_elems = date_range_modal_elem.select(selector!("div#dateRangeModal a"));
        let mut schedules = Vec::new();
        for (index, schedule_path_query_elem) in schedule_path_query_elems.enumerate() {
            let schedule_path_query_text = schedule_path_query_elem.value().attr("href").ok_or_else(|| {
                anyhow!("Missing schedule path/query in date range link element: {}", schedule_path_query_elem.html())
            })?;
            let opt_schedule = if index == 0 {
                scrape_schedule(options, &base_url, &base_document, terminal_pair, index, today).await?
            } else {
                let source_url = format!("{}{}", BCFERRIES_BASE_URL, schedule_path_query_text);
                let document = cache
                    .get_html(&source_url, &HTML_ERROR_REGEX)
                    .await
                    .with_context(|| format!("Failed to download schedule HTML from: {:?}", source_url))?;
                scrape_schedule(options, &source_url, &document, terminal_pair, index, today).await?
            };
            opt_schedule.iter().for_each(|s| debug!("Parsed schedule: {:#?}", s));
            schedules.extend(opt_schedule);
        }
        ensure!(!schedules.is_empty(), "Failed to find any schedule elements");
        Ok(schedules) as Result<_>
    };
    inner.await.with_context(|| format!("Failed to scrape route schedule for {} from: {:?}", terminal_pair, base_url))
}

pub async fn scrape_schedules(options: &Options, cache: &Cache<'_>) -> Result<Vec<Schedule>> {
    let inner = async {
        let today = today_vancouver();
        let mut result = Vec::new();
        for &terminal_pair in ALL_TERMINAL_PAIRS.iter() {
            result.extend(scrape_route_schedules(options, cache, terminal_pair, today).await?);
        }
        Ok(result) as Result<_>
    };
    inner.await.context("Failed to scrape schedules")
}
