use crate::annotations::*;
use crate::cache::*;
use crate::constants::*;
use crate::depart_time_and_row_annotations::*;
use crate::imports::*;
use crate::macros::*;
use crate::types::*;
use crate::utils::*;
use crate::weekday_dates::*;

fn parse_annotations(table_elem: &ElementRef, date_range: &DateRange) -> Result<(Annotations, Vec<Vec<String>>)> {
    let inner = || {
        let mut annotations = Annotations::new();
        let mut item_rows = Vec::new();
        for row_elem in table_elem.select(selector!("tr:not(:first-child)")) {
            let cell_elems: Vec<ElementRef> = row_elem.select(selector!("td")).collect();
            if cell_elems.len() == 4 {
                let item_row: Vec<_> = cell_elems.iter().map(element_text).collect();
                if !item_row.iter().all(String::is_empty) {
                    item_rows.push(item_row);
                }
            } else if cell_elems.len() == 1 || (cell_elems.len() == 2 && element_text(&cell_elems[1]).is_empty()) {
                let annotation_texts = element_texts(&cell_elems[0]);
                annotations.parse(annotation_texts, date_range)?;
            } else {
                bail!(
                    "Expect schedule row to have either 1 or 4 cells; found {:?} in: {}",
                    cell_elems.len(),
                    row_elem.html()
                );
            }
        }
        Ok((annotations, item_rows))
    };
    inner().context("Failed to parse annotations")
}

fn parse_items(
    item_rows: Vec<Vec<String>>,
    annotations: &Annotations,
    date_range: &DateRange,
) -> Result<Vec<ScheduleItem>> {
    let mut items = Vec::new();
    for cell_texts in item_rows {
        let mut inner = || {
            let DepartTimeAndRowAnnotations { time: depart_time, row_dates, row_notes } =
                DepartTimeAndRowAnnotations::parse(&cell_texts[0], annotations)?;
            let weekday_dates = WeekdayDates::parse(&cell_texts[1], annotations, date_range)?;
            let weekdays = weekday_dates.to_date_restrictions(&row_dates);
            let stops_text = &cell_texts[2];
            let stops = parse_schedule_stops(stops_text.split(','))
                .with_context(|| format!("Failed to parse stops: {:?}", stops_text))?;
            let arrive_time_text = &cell_texts[3];
            let arrive_time = parse_schedule_time(arrive_time_text)
                .with_context(|| format!("Failed to parse arrive time: {:?}", arrive_time_text))?;
            let sailing = Sailing { depart_time, arrive_time, stops };
            let notes = AnnotationDates::map_to_date_restrictions_by_weekdays(row_notes, &weekdays);
            items.push(ScheduleItem { sailing, weekdays, notes });
            Ok(()) as Result<_>
        };
        inner().with_context(|| format!("Failed to parse schedule row: {:?}", cell_texts))?
    }
    Ok(items)
}

fn parse_date_range(text: &str) -> Result<DateRange> {
    DateRange::parse(text, "%B %e, %Y", " - ")
        .with_context(|| format!("Failed to parse schedule HTML date range: {:?}", text))
}

fn parse_schedule(
    options: &Options,
    terminal_pair: TerminalCodePair,
    date_range_elem: &ElementRef,
    today: NaiveDate,
    source_url: &str,
) -> Result<Option<Schedule>> {
    let date_range_text = element_text(date_range_elem);
    let inner = || {
        let date_range = parse_date_range(&date_range_text)
            .with_context(|| format!("Failed to parse date range: {:?}", date_range_text))?;
        if !should_scrape_schedule_date(date_range, today, options.date) {
            return Ok(None);
        }
        info!("Parsing schedule for {}, {}", terminal_pair, date_range);
        let table_container_elem = date_range_elem
            .parent_element()
            .unwrap()
            .next_sibling_element()
            .ok_or_else(|| anyhow!("Expect schedule table container element after schedule date range element"))?;
        let mut table_elems = table_container_elem.select(selector!("div.component-cnrl > table"));
        if let Some(table_elem) = table_elems.next() {
            if table_elems.next().is_some() {
                bail!("Expect zero or one tables in schedule table container element");
            }
            let (annotations, item_rows) = parse_annotations(&table_elem, &date_range)?;
            let items = parse_items(item_rows, &annotations, &date_range)?;
            Ok(Some(Schedule {
                terminal_pair,
                date_range,
                items,
                source_url: format!("{}#{}", source_url, terminal_pair.from),
            }))
        } else {
            Ok(None)
        }
    };
    inner().with_context(|| format!("Failed to parse schedule for {}, {}", terminal_pair, date_range_text))
}

pub async fn scrape_non_tsawwassen_schedules(
    options: &Options,
    cache: &Cache<'_>,
    today: NaiveDate,
) -> Result<Vec<Schedule>> {
    const SOURCE_URL: &str = "https://www.bcferries.com/routes-fares/schedules/southern-gulf-islands";
    let inner = async {
        let document = cache
            .get_html(SOURCE_URL, &IGNORE_HTML_CHANGES_REGEX)
            .await
            .with_context(|| format!("Failed to download schedule HTML from: {:?}", SOURCE_URL))?;
        let mut schedules = Vec::new();
        for terminal_pair_description_elem in document.select(selector!("div.js-accordion > h4")) {
            let terminal_pair_id = terminal_pair_description_elem.value().id().ok_or_else(|| {
                anyhow!("Terminal pair element missing ID: {}", terminal_pair_description_elem.html())
            })?;
            let terminal_pair = TerminalCodePair::parse_schedule_code_pair(terminal_pair_id)
                .with_context(|| format!("Failed to parse terminal pair element ID: {:?}", terminal_pair_id))?;
            if options.terminals.is_some() && options.terminals != Some(terminal_pair) {
                continue;
            }
            let schedule_container_elem = terminal_pair_description_elem.parent_element().unwrap();
            let schedule_date_range_elems = schedule_container_elem.select(selector!("header > span.accordion-title"));
            for schedule_date_range_elem in schedule_date_range_elems {
                let opt_schedule =
                    parse_schedule(options, terminal_pair, &schedule_date_range_elem, today, SOURCE_URL)?;
                opt_schedule.iter().for_each(|s| debug!("Parsed schedule: {:#?}", s));
                schedules.extend(opt_schedule);
            }
        }
        Ok(schedules) as Result<_>
    };
    inner.await.with_context(|| format!("Failed to scrape non-Tsawwassen schedules from: {:?}", SOURCE_URL))
}
