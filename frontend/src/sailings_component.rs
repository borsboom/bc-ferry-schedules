use crate::imports::*;
use crate::sailings_processor::*;
use crate::types::*;
use crate::utils::*;

#[derive(Eq, PartialEq, Properties)]
pub struct SailingsProps {
    pub area_pair: AreaPair,
    pub date: Option<Date>,
}

struct DateInputState {
    input: String,
    value: StdResult<Date, &'static str>,
}

enum SailingsStateModel<'a> {
    InvalidDate(String),
    LoadingSchedules,
    LoadSchedulesFailed,
    NoSchedule,
    NoSailings,
    Sailings(Vec<(&'a Schedule, Vec<SailingWithNotes>)>),
}

struct SailingsModel<'a> {
    sailings_state_model: SailingsStateModel<'a>,
    area_pair: AreaPair,
    view_date: Date,
    max_date: Date,
}

struct FormModel {
    history: AnyHistory,
    date_input_state: UseStateHandle<DateInputState>,
    area_pair: AreaPair,
    query_date: Option<Date>,
    today: Date,
    view_date: Date,
    max_date: Date,
}

struct InformationUrlsModel<'a> {
    sailing_status_url: &'a str,
    departures_url: &'a str,
    service_notices_url: &'a str,
}

fn stop_html(stop: &Stop) -> Html {
    html! {
        <li>
        { match stop.type_ {
            StopType::Stop => "Stop",
            StopType::Transfer => "Transfer",
            StopType::Thrufare => "Thru-fare",
        }}
        { " " }
        { stop.terminal.area().short_name() }
        </li>
    }
}

fn alert_row_html(alert: &Alert) -> Html {
    let alert_class = match &alert.level {
        AlertLevel::Info => "alert-info",
        AlertLevel::Warning => "alert-warning",
        AlertLevel::Danger => "alert-danger",
    };
    html! {
        <tr>
            <td colspan="3" class="border-bottom-0">
                <div class={ classes!("alert", alert_class, "mb-0") }>
                    { &alert.message }
                </div>
            </td>
        </tr>
    }
}

fn sailing_row_html(sailing: &SailingWithNotes) -> Html {
    let main_td_class = (!sailing.notes.is_empty()).then_some("border-bottom-0");
    let all_td_class = sailing.sailing.is_thrufare().then_some("text-muted");
    html! { <>
        <tr>
            <td class={ classes!(all_td_class, main_td_class) }>{ format_time(sailing.sailing.depart_time) }</td>
            <td class={ classes!(all_td_class, main_td_class) }>{ format_time(sailing.sailing.arrive_time) }</td>
            <td class={ classes!("text-nowrap", all_td_class, main_td_class) }>
                { if sailing.sailing.stops.is_empty() { html! {
                    <span class="text-muted">{ "non-stop" }</span>
                }} else { html! {
                    <ul class="list-unstyled mb-0">
                        { for sailing.sailing.stops.iter().map(stop_html) }
                    </ul>
                }}}
            </td>
        </tr>
        { if !sailing.notes.is_empty() { html! {
            <tr>
                <td colspan="3" class={ classes!("small", "pt-0", all_td_class) }>
                    <ul class="mb-0">
                        { for sailing.notes.iter().map(|note| { html! {
                            <li>{ note }</li>
                        }})}
                    </ul>
                </td>
            </tr>
        }} else {
            html! {}
        }
    }</>}
}

fn schedule_sailings_header_row_html(schedule: &Schedule) -> Html {
    html! {
        <tr>
            <th class="bg-heading">
                <span class="fw-normal">{ "Depart " }</span>
                <span class="text-nowrap">{ schedule.terminal_pair.from.name() }</span>
            </th>
            <th class="bg-heading">
                <span class="fw-normal">{ "Arrive " }</span>
                <span class="text-nowrap">{ schedule.terminal_pair.to.name() }</span>
            </th>
            <th class="bg-heading fw-normal">
                { "Stops" }
            </th>
        </tr>
    }
}

fn schedule_sailings_rows_html(first: bool, last: bool, schedule: &Schedule, sailings: &[SailingWithNotes]) -> Html {
    let bottom_class = (!last).then_some("pb-3");
    html! { <>
        { if first {
            html! {
                <thead class="table-dark">
                    { schedule_sailings_header_row_html(schedule) }
                </thead>
            }
        } else {
            html! {
                <tbody class="table-dark">
                    { schedule_sailings_header_row_html(schedule) }
                </tbody>
            }
        }}
        <tbody>
        { for schedule.alerts.iter().map(alert_row_html) }
        { for sailings.iter().map(sailing_row_html) }
        </tbody>
        <tbody>
            <tr>
                <td colspan=3 class={classes!("text-end", "text-muted", "d-print-none", "border-bottom-0", "p-0", "bg-transparent", bottom_class)}>
                    <small>
                        { "Data updated " }
                        { human_time(schedule.refreshed_at) }
                        { " from " }
                        <a class="link-secondary" href={ schedule.source_url.clone() } target="_blank">
                            { "original schedule" }
                        </a>
                    </small>
                </td>
            </tr>
        </tbody>
    </> }
}

impl<'a> SailingsModel<'a> {
    fn new(
        schedules_state: &'a SchedulesState,
        date_input_state: &DateInputState,
        area_pair: AreaPair,
        query_date_or_today: Date,
    ) -> SailingsModel<'a> {
        let base = SailingsModel {
            sailings_state_model: SailingsStateModel::NoSailings,
            area_pair,
            view_date: query_date_or_today,
            max_date: query_date_or_today,
        };
        match (date_input_state.value, schedules_state) {
            (Err(err), _) => {
                SailingsModel { sailings_state_model: SailingsStateModel::InvalidDate(err.to_string()), ..base }
            }
            (Ok(view_date), SchedulesState::Init) | (Ok(view_date), SchedulesState::Loading) => SailingsModel {
                sailings_state_model: SailingsStateModel::LoadingSchedules,
                view_date,
                max_date: view_date,
                ..base
            },
            (Ok(view_date), SchedulesState::Failed) => SailingsModel {
                sailings_state_model: SailingsStateModel::LoadSchedulesFailed,
                view_date,
                max_date: view_date,
                ..base
            },
            (Ok(view_date), SchedulesState::Loaded(schedules_map)) => {
                let max_date = max(
                    view_date,
                    AREA_PAIR_TERMINAL_PAIRS
                        .get(&area_pair)
                        .and_then(|tps| {
                            tps.iter()
                                .flat_map(|tp| {
                                    schedules_map.get(tp).and_then(|ss| ss.iter().map(|s| s.date_range.to).max())
                                })
                                .max()
                        })
                        .unwrap_or(view_date),
                );
                if let Some(schedules_sailings) = area_sailings_for_date(area_pair, view_date, schedules_map) {
                    if schedules_sailings.is_empty() {
                        SailingsModel {
                            sailings_state_model: SailingsStateModel::NoSailings,
                            view_date,
                            max_date,
                            ..base
                        }
                    } else {
                        SailingsModel {
                            sailings_state_model: SailingsStateModel::Sailings(schedules_sailings),
                            view_date,
                            max_date,
                            ..base
                        }
                    }
                } else {
                    SailingsModel { sailings_state_model: SailingsStateModel::NoSchedule, view_date, max_date, ..base }
                }
            }
        }
    }

    fn sailings_table_html(&self, schedule_sailings: &[(&Schedule, Vec<SailingWithNotes>)]) -> Html {
        let last_schedule_index = schedule_sailings.len() - 1;
        html! { <>
            <div>
                <h6>{ self.view_date.format(format_description!("[weekday], [day padding:none] [month repr:long], [year]")).expect("friendly date to format") }</h6>
            </div>
            <table class="table table-light mb-0">
                { for schedule_sailings.iter().enumerate().map(|(index, (schedule, sailings))|
                    schedule_sailings_rows_html(index == 0, index == last_schedule_index, schedule, sailings)
                ) }
            </table>
        </> }
    }

    fn sailings_html(&self) -> Html {
        match &self.sailings_state_model {
            SailingsStateModel::InvalidDate(err) => html! {
                <div class="alert alert-danger text-center">{ err }</div>
            },
            SailingsStateModel::LoadingSchedules => html! {
                <div class="alert alert-light border text-center">
                    <div class="spinner-border" role="status"/>
                    <div>{ "Loading schedules..." }</div>
                </div>
            },
            SailingsStateModel::LoadSchedulesFailed => html! {
                <div class="alert alert-danger text-center" role="alert">
                    { "There was a problem loading the ferry schedules; please refresh your browser to try again." }
                </div>
            },
            SailingsStateModel::NoSchedule => html! {
                <div class="alert alert-warning text-center" role="alert">
                    { "There is no schedule available for this date yet; please check back later!" }
                </div>
            },
            SailingsStateModel::NoSailings => html! {
                <div class="alert alert-light border text-center" role="alert">
                    { "There are no sailings between the these terminals on the specified date." }
                </div>
            },
            SailingsStateModel::Sailings(schedule_sailings) => self.sailings_table_html(schedule_sailings),
        }
    }

    fn html(self) -> Html {
        let info_urls = if self.area_pair.includes_terminal(Terminal::SWB)
            && self.area_pair.includes_any_terminal(&*ROUTE5_GULF_ISLAND_TERMINALS)
        {
            InformationUrlsModel {
                sailing_status_url: SWB_SGI_SAILING_STATUS_URL,
                departures_url: SWB_DEPARTURES_URL,
                service_notices_url: SWB_SGI_SERVICE_NOTICES_URL,
            }
        } else if self.area_pair.includes_terminal(Terminal::TSA)
            && self.area_pair.includes_any_terminal(&*ROUTE5_GULF_ISLAND_TERMINALS)
        {
            InformationUrlsModel {
                sailing_status_url: TSA_SGI_SAILING_STATUS_URL,
                departures_url: TSA_DEPARTURES_URL,
                service_notices_url: TSA_SGI_SERVICE_NOTICES_URL,
            }
        } else {
            InformationUrlsModel {
                sailing_status_url: ALL_SAILING_STATUS_URL,
                departures_url: ALL_DEPARTURES_URL,
                service_notices_url: ALL_SERVICE_NOTICES_URL,
            }
        };
        let is_reservable = self.area_pair.is_reservable();
        let has_thrufares = self.area_pair.has_thrufares();
        html! { <>
            <div class="row mt-4">
                <div class="col-12 col-md-8 col-lg-6">
                    { self.sailings_html() }
                </div>
            </div>
            { if is_reservable || has_thrufares { html! { <>
                <div class="mt-3">
                    <small>
                        { if is_reservable { html! {
                            <span class="text-nowrap">
                                <a href={ BCFERRIES_HOME_URL } target="_blank">{ "Reservations" }</a>
                                { " are recommended for direct sailings." }
                            </span>
                        }} else {
                            html! {}
                        }}
                        { if has_thrufares { html! { <>
                            { if is_reservable { " " } else { "" }}
                            <span class="text-nowrap">
                                { "See here for more " }
                                <a href={ THRU_FARE_INFORMATION_URL } target="_blank">{ "information about thru-fares" }</a>
                                { "." }
                            </span>
                        </> }} else {
                            html! {}
                        }}
                        </small>
                </div>
            </> }} else {
                html! {}
            }}
            <div class="mt-3 text-muted">
                <small>
                    <div><strong>{ "BC Ferries may adjust schedules at any time and without notice." }</strong></div>
                    <div>
                        { "Confirm all sailings with the original schedule" }
                        { ", and check " }
                        <a class="link-secondary" href={ info_urls.service_notices_url } target="_blank">
                            { "service notices" }
                        </a>
                        { ", " }
                        <a class="link-secondary" href={ info_urls.departures_url } target="_blank">
                            { "departures" }
                        </a>
                        { " and " }
                        <a class="link-secondary" href={ info_urls.sailing_status_url } target="_blank">
                            { "sailing status" }
                        </a>
                        { " before you depart." }
                        { " If you find a mistake, send feedback to " }
                        <a class="link-secondary" href="mailto:emanuel@borsboom.io" target="_blank">{ "emanuel@borsboom.io" }</a>
                        { "." }
                    </div>
                </small>
            </div>
        </> }
    }
}

impl FormModel {
    fn onchange_date_input_callback(&self) -> Callback<Event> {
        let date_input_state = self.date_input_state.clone();
        let history = self.history.clone();
        let area_pair = self.area_pair;
        let today = self.today;
        Callback::once(move |e: Event| {
            let orig_date_input = e.target_unchecked_into::<HtmlInputElement>().value();
            let trimmed_date_input = orig_date_input.trim();
            if trimmed_date_input.is_empty() {
                date_input_state.set(DateInputState { input: format_iso8601_date(today), value: Ok(today) });
                history
                    .push_with_query(
                        Route::Sailings,
                        SailingsQuery { from: Some(area_pair.from), to: Some(area_pair.to), date: None },
                    )
                    .expect("history to push");
            } else if let Ok(date) = parse_iso8601_date(trimmed_date_input) {
                if date < today {
                    date_input_state.set(DateInputState {
                        input: orig_date_input.to_owned(),
                        value: Err("Date may not be in the past."),
                    });
                } else {
                    date_input_state.set(DateInputState { input: format_iso8601_date(date), value: Ok(date) });
                    history
                        .push_with_query(
                            Route::Sailings,
                            SailingsQuery { from: Some(area_pair.from), to: Some(area_pair.to), date: Some(date) },
                        )
                        .expect("history to push");
                }
            } else {
                date_input_state.set(DateInputState {
                    input: orig_date_input.to_owned(),
                    value: Err("Date format must be YYYY-MM-DD."),
                });
            }
        })
    }

    fn onclick_adjust_date_button_callback(&self, opt_new_date: Option<Date>) -> Callback<MouseEvent> {
        let date_input_state = self.date_input_state.clone();
        let history = self.history.clone();
        let area_pair = self.area_pair;
        let today = self.today;
        let new_date = opt_new_date.unwrap_or(today);
        Callback::once(move |_| {
            date_input_state.set(DateInputState { input: format_iso8601_date(new_date), value: Ok(new_date) });
            history
                .push_with_query(
                    Route::Sailings,
                    SailingsQuery { from: Some(area_pair.from), to: Some(area_pair.to), date: opt_new_date },
                )
                .expect("history to push");
        })
    }

    fn onclick_swap_terminals_button_callback(&self) -> Callback<MouseEvent> {
        let history = self.history.clone();
        let area_pair = self.area_pair.swapped();
        let query_date = self.query_date;
        Callback::once(move |_| {
            history
                .push_with_query(
                    Route::Sailings,
                    SailingsQuery { from: Some(area_pair.from), to: Some(area_pair.to), date: query_date },
                )
                .expect("history to push");
        })
    }

    fn html(self) -> Html {
        html! {
            <div class="d-print-none">
                <div class="row mb-1">
                    <label class="col-2 col-md-1 col-form-label">{ "From" }</label>
                    <div class="col-10 col-md-7 col-lg-5">
                        <span class="form-control">
                            <strong>
                                { area_link_html(
                                    self.area_pair.from,
                                    SailingsQuery{ from: None, to: Some(self.area_pair.to), date: self.query_date }
                                ) }
                            </strong>
                        </span>
                    </div>
                </div>
                <div class="row mb-1">
                    <label class="col-2 col-md-1 col-form-label">{ "To" }</label>
                    <div class="col-10 col-md-7 col-lg-5">
                        <span class="form-control">
                            <strong>
                                { area_link_html(
                                    self.area_pair.to,
                                    SailingsQuery{ from: Some(self.area_pair.from), to: None, date: self.query_date }
                                ) }
                            </strong>
                        </span>
                    </div>
                </div>
                <div class="row mb-3">
                    <label for="date-input" class="col-2 col-md-1 col-form-label">{ "Date" }</label>
                    <div class="col-10 col-md-7 col-lg-5 d-flex">
                        <input
                            id="date-input"
                            type="date"
                            placeholder="YYYY-MM-DD"
                            required={ true }
                            class={ classes!("form-control", "align-self-center", "date-input", self.date_input_state.value.is_err().then_some("is-invalid")) }
                            value={ self.date_input_state.input.to_owned() }
                            min={ format_iso8601_date(self.today) }
                            max={ format_iso8601_date(self.max_date) }
                            onchange={ self.onchange_date_input_callback() }/>
                        <button
                            type="button"
                            class="btn btn-outline-secondary border-0 pe-0"
                            title="Next Date"
                            onclick={ self.onclick_adjust_date_button_callback(Some(max(self.view_date.previous_day().expect("view date to have previous date"), self.today))) }
                            disabled={ self.date_input_state.value.as_ref().map(|d| *d <= self.today).unwrap_or(true) }
                        >
                            <i class="bi bi-caret-left-fill"/>
                        </button>
                        <button
                            type="button"
                            class="btn btn-outline-secondary border-0 ps-0"
                            title="Previous Date"
                            onclick={ self.onclick_adjust_date_button_callback(Some(min(self.view_date.next_day().expect("view date to have next day"), self.max_date))) }
                            disabled={ self.date_input_state.value.as_ref().map(|d| *d >= self.max_date).unwrap_or(true) }
                        >
                            <i class="bi bi-caret-right-fill"/>
                        </button>
                        <button
                            type="button"
                            class="btn btn-outline-secondary border-0"
                            title="Today"
                            onclick={ self.onclick_adjust_date_button_callback(None) }
                            disabled={ self.query_date.is_none() }
                        >
                            <i class="bi bi-x-circle"/>
                        </button>
                        <span class="me-auto"/>
                        <button
                            type="button"
                            class="btn btn-outline-secondary btn-sm mb-1 d-print-none"
                            title="Switch Direction"
                            onclick={ self.onclick_swap_terminals_button_callback() }
                        >
                            <i class="bi bi-arrow-left-right"/>
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}

#[function_component(Sailings)]
pub fn sailings_component(props: &SailingsProps) -> Html {
    let area_pair = AreaPair { from: props.area_pair.from, to: props.area_pair.to };
    let query_date = props.date;
    let today = today_vancouver();
    let query_date_or_today = match query_date {
        None => today,
        Some(date) if date < today => today,
        Some(date) => date,
    };
    let history = use_history().expect("history to be available");
    let schedules_state = use_context::<SchedulesState>().expect("schedules state to be available");
    let date_input_state = use_state(|| DateInputState {
        input: format_iso8601_date(query_date_or_today),
        value: Ok(query_date_or_today),
    });
    let sailings_model = SailingsModel::new(&schedules_state, &date_input_state, area_pair, query_date_or_today);
    let form_model = FormModel {
        history,
        date_input_state,
        area_pair,
        query_date,
        today,
        view_date: sailings_model.view_date,
        max_date: sailings_model.max_date,
    };
    html! { <>
        { form_model.html() }
        { sailings_model.html() }
    </> }
}
