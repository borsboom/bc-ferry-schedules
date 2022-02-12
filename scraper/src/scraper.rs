use crate::annotations::*;
use crate::cache::*;
use crate::imports::*;
use crate::types::*;
use crate::utils::*;
use crate::weekday_restrictions::*;
use ::scraper::{ElementRef, Selector};
use ::selectors::Element;

fn parse_schedule_time(text: &str) -> Result<NaiveTime> {
    if let Ok(time) = NaiveTime::parse_from_str(text, "%l:%M %p") {
        Ok(time)
    } else {
        let time = NaiveTime::parse_from_str(text, "%l;%M %p")?;
        Ok(time)
    }
}

fn parse_annotations<'a>(
    schedule_table_elem: &ElementRef<'a>,
    effective_date_range: &DateRange,
) -> Result<(Annotations, Vec<Vec<ElementRef<'a>>>)> {
    let mut annotations = Annotations::new();
    let mut item_rows = Vec::new();
    for schedule_row_elem in schedule_table_elem.select(selector!("tr:not(:first-child)")) {
        let schedule_cell_elems: Vec<ElementRef> = schedule_row_elem.select(selector!("td")).collect();
        if schedule_cell_elems.len() == 4 {
            item_rows.push(schedule_cell_elems);
        } else if schedule_cell_elems.len() == 1
            || (schedule_cell_elems.len() == 2 && element_text(schedule_cell_elems.get(1).unwrap()).is_empty())
        {
            let annotation_texts = element_texts(schedule_cell_elems.get(0).unwrap());
            annotations.parse(&annotation_texts, effective_date_range)?;
        } else {
            bail!(
                "Schedule row should have either 1 or 4 cells; found {:?}: {}",
                schedule_cell_elems.len(),
                schedule_row_elem.html()
            );
        }
    }
    Ok((annotations, item_rows))
}

fn parse_depart_time(
    elem: &ElementRef,
    annotations: &Annotations,
) -> Result<(NaiveTime, DateRestriction, Vec<&'static str>)> {
    let starstar_suffix_re: &Regex = regex!(r" ?\*\*$");
    let exclamation_suffix_re: &Regex = regex!(r" ?!$");
    let hash_suffix_re: &Regex = regex!(r" ?#$");
    let plus_suffix_re: &Regex = regex!(r" ?\+$");
    let exclamation_plus_suffix_re: &Regex = regex!(r" ?! ?\+$");
    let orig_text = element_text(elem);
    let mut row_date_restriction = DateRestriction::new();
    let mut row_text_annotations = Vec::new();
    let text = if exclamation_plus_suffix_re.is_match(&orig_text) {
        row_date_restriction.extend(&annotations.exclamation);
        row_date_restriction.extend(&annotations.plus);
        row_text_annotations.extend(&annotations.exclamation_text);
        row_text_annotations.extend(&annotations.plus_text);
        exclamation_plus_suffix_re.replace(&orig_text, "")
    } else if starstar_suffix_re.is_match(&orig_text) {
        row_date_restriction.extend(&annotations.starstar);
        starstar_suffix_re.replace(&orig_text, "")
    } else if exclamation_suffix_re.is_match(&orig_text) {
        row_date_restriction.extend(&annotations.exclamation);
        row_text_annotations.extend(&annotations.exclamation_text);
        exclamation_suffix_re.replace(&orig_text, "")
    } else if hash_suffix_re.is_match(&orig_text) {
        row_date_restriction.extend(&annotations.hash);
        row_text_annotations.extend(&annotations.hash_text);
        hash_suffix_re.replace(&orig_text, "")
    } else if plus_suffix_re.is_match(&orig_text) {
        row_date_restriction.extend(&annotations.plus);
        row_text_annotations.extend(&annotations.plus_text);
        plus_suffix_re.replace(&orig_text, "")
    } else {
        Cow::from(&orig_text)
    };
    let depart_time =
        parse_schedule_time(&text).with_context(|| format!("Invalid depart time in {:?}: {:?}", orig_text, text))?;
    Ok((depart_time, row_date_restriction, row_text_annotations))
}

fn parse_stops(elem: &ElementRef) -> Result<Vec<Stop>> {
    let text = element_text(elem);
    if text == "non-stop" {
        Ok(vec![])
    } else {
        text.split(',').map(|s| Stop::from_schedule_text(s.trim())).collect()
    }
}

fn parse_items(
    item_rows: Vec<Vec<ElementRef>>,
    annotations: &Annotations,
    effective_date_range: &DateRange,
) -> Result<Vec<ScheduleItem>> {
    let mut items = Vec::new();
    for schedule_cell_elems in item_rows {
        let (depart_time, row_date_restriction, row_text_annotations) =
            parse_depart_time(schedule_cell_elems.get(0).unwrap(), annotations)?;
        let weekday_restrictions = WeekdayRestrictions::parse(
            &element_text(schedule_cell_elems.get(1).unwrap()),
            annotations,
            effective_date_range,
        )?;
        let (weekdays, except_dates) = weekday_restrictions.into_schedule_weekdays(row_date_restriction)?;
        let stops = parse_stops(schedule_cell_elems.get(2).unwrap())?;
        let arrive_time_text = element_text(schedule_cell_elems.get(3).unwrap());
        let arrive_time = NaiveTime::parse_from_str(&arrive_time_text, "%l:%M %p")
            .with_context(|| format!("Invalid arrive time: {:?}", arrive_time_text))?;
        let sailing = Sailing { depart_time, arrive_time, stops, annotations: row_text_annotations };
        items.push(ScheduleItem { sailing, except_dates, weekdays });
    }
    Ok(items)
}

fn parse_schedule(
    terminal_pair: TerminalPair,
    schedule_date_range_elem: &ElementRef,
    today: NaiveDate,
    source_url: &str,
) -> Result<Option<Schedule>> {
    let effective_date_range = DateRange::from_schedule_text(&element_text(schedule_date_range_elem))?;
    if effective_date_range.to < today {
        debug!("Skipping old schedule: {}, {}", terminal_pair, effective_date_range);
        return Ok(None);
    }
    info!("Parsing schedule: {}, {}", terminal_pair, effective_date_range);
    let schedule_table_container_elem =
        schedule_date_range_elem.parent_element().unwrap().next_sibling_element().ok_or_else(|| {
            anyhow!(
                "Expected schedule table container element after schedule date range element: {}",
                schedule_date_range_elem.html()
            )
        })?;
    let mut schedule_table_elems = schedule_table_container_elem.select(selector!("div.component-cnrl > table"));
    if let Some(schedule_table_elem) = schedule_table_elems.next() {
        if schedule_table_elems.next().is_some() {
            bail!(
                "Expected zero or one tables in schedule table container element: {}",
                schedule_date_range_elem.html()
            );
        }
        let (annotations, item_rows) = parse_annotations(&schedule_table_elem, &effective_date_range)?;
        let items = parse_items(item_rows, &annotations, &effective_date_range)?;
        Ok(Some(Schedule {
            terminal_pair,
            effective_date_range,
            items,
            source_url: source_url.to_string(),
            route_group: RouteGroup::SaltSpringAndOuterGulfIslands,
            reservable: false,
        }))
    } else {
        Ok(None)
    }
}

pub async fn scrape_non_tsawwassen_schedules(cache: &Cache<'_>) -> Result<Vec<Schedule>> {
    const SOURCE_URL: &str = "https://www.bcferries.com/routes-fares/schedules/southern-gulf-islands";
    let today = now_pacific().date().naive_local();
    let document = match cache
        .get_html(SOURCE_URL, regex!(r"ACC\.config\.CSRFToken =.*|ACC\.addons\.liveeditaddon\['liveeditaddon\.message\.slot\.tooltip\.action\..*"))
        .await
        .with_context(|| format!("Could not download schedule from: {:?}", SOURCE_URL))?
    {
        Cached::Unchanged => {
            info!("Source data is unchanged.");
            return Ok(vec![]);
        }
        Cached::Contents(contents) => contents,
    };
    let mut schedules = Vec::new();
    for terminal_pair_description_elem in document.select(selector!("div.js-accordion > h4")) {
        let terminal_pair_elem_id =
            terminal_pair_description_elem.value().id().ok_or_else(|| anyhow!("Element missing ID"))?;
        let terminal_pair = TerminalPair::from_schedule_codes(terminal_pair_elem_id).with_context(|| {
            format!("Invalid terminal pair description element: {}", terminal_pair_description_elem.html())
        })?;
        let schedule_container_elem = terminal_pair_description_elem.parent_element().unwrap();
        let schedule_date_range_elems = schedule_container_elem.select(selector!("header > span.accordion-title"));
        for schedule_date_range_elem in schedule_date_range_elems {
            let opt_schedule = parse_schedule(terminal_pair, &schedule_date_range_elem, today, SOURCE_URL)
                .with_context(|| {
                    format!(
                        "Unable to parse schedule for {}, {}",
                        terminal_pair,
                        element_text(&schedule_date_range_elem)
                    )
                })?;
            if let Some(schedule) = opt_schedule {
                debug!("Parsed schedule: {:#?}", schedule);
                schedules.push(schedule);
            }
        }
    }
    Ok(schedules)
}
