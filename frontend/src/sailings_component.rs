use crate::imports::*;
use crate::sailings_processor::*;
use crate::types::*;
use crate::utils::*;

const DEFAULT_SCHEDULE_SOURCE_URL: &str = "https://www.bcferries.com/routes-fares/schedules";

#[derive(Properties, PartialEq)]
pub struct SailingsProps {
    pub terminal_pair: TerminalCodePair,
    pub date: Option<Date>,
}

struct DateInputState {
    input: String,
    value: StdResult<Date, &'static str>,
}

enum SailingsStateModel {
    InvalidDate(String),
    LoadingSchedules,
    LoadSchedulesFailed,
    NoSchedule,
    NoSailings,
    Sailings(Vec<SailingWithNotes>),
}

struct SailingsModel<'a> {
    sailings_state_model: SailingsStateModel,
    terminal_pair: TerminalCodePair,
    source_schedule: Option<&'a Schedule>,
    view_date: Date,
    max_date: Date,
}

struct FormModel {
    history: AnyHistory,
    date_input_state: UseStateHandle<DateInputState>,
    terminal_pair: TerminalCodePair,
    query_date: Option<Date>,
    today: Date,
    view_date: Date,
    max_date: Date,
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
        { stop.terminal.short_location_name() }
        </li>
    }
}

fn sailing_row_html(sailing: &SailingWithNotes) -> Html {
    let main_td_class = (!sailing.notes.is_empty()).then(|| "border-bottom-0");
    let all_td_class = sailing.sailing.is_thrufare().then(|| "text-muted");
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

impl<'a> SailingsModel<'a> {
    fn new(
        schedules_state: &'a SchedulesState,
        date_input_state: &DateInputState,
        terminal_pair: TerminalCodePair,
        query_date_or_today: Date,
    ) -> SailingsModel<'a> {
        let base = SailingsModel {
            sailings_state_model: SailingsStateModel::NoSailings,
            terminal_pair,
            source_schedule: None,
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
                    schedules_map
                        .get(&terminal_pair)
                        .and_then(|schedules| schedules.iter().map(|s| s.date_range.to).max())
                        .unwrap_or(view_date),
                );
                if let Some((schedule, sailings)) = sailings_for_date(terminal_pair, view_date, schedules_map) {
                    if sailings.is_empty() {
                        SailingsModel {
                            sailings_state_model: SailingsStateModel::NoSailings,
                            view_date,
                            max_date,
                            ..base
                        }
                    } else {
                        SailingsModel {
                            sailings_state_model: SailingsStateModel::Sailings(sailings),
                            source_schedule: Some(schedule),
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

    fn sailings_table_html(&self, sailings: &[SailingWithNotes]) -> Html {
        html! { <>
            <div>
                <h6>{ self.view_date.format(format_description!("[weekday], [day padding:none] [month repr:long], [year]")).unwrap() }</h6>
            </div>
            <table class="table table-light mb-0">
                <thead class="table-dark">
                    <tr>
                        <th class="bg-heading">{ "Depart " }<span class="text-nowrap">{ self.terminal_pair.from.short_location_name() }</span></th>
                        <th class="bg-heading">{ "Arrive " }<span class="text-nowrap">{ self.terminal_pair.to.short_location_name() }</span></th>
                        <th class="bg-heading">{ "Stops" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for sailings.iter().map(sailing_row_html) }
                </tbody>
            </table>
            { if let Some(schedule) = self.source_schedule { html! {
                <div class="d-flex flex-column align-items-end text-muted d-print-none">
                    <small>
                        { "Data updated " }
                        { human_time(schedule.refreshed_at) }
                        { " from " }
                        <a class="link-secondary" href={ schedule.source_url.clone() } target="_blank">
                            { "original schedule" }
                        </a>
                    </small>
                </div>
            }} else {
                html! {}
            }}
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
            SailingsStateModel::Sailings(sailings) => self.sailings_table_html(sailings),
        }
    }

    fn html(self) -> Html {
        let source_url = self
            .source_schedule
            .map(|s| s.source_url.clone())
            .unwrap_or_else(|| DEFAULT_SCHEDULE_SOURCE_URL.to_string());
        let (sailing_status_url, departures_url, service_notices_url) = self.source_schedule.and_then(|schedule| {
            if schedule.terminal_pair.includes_terminal(TerminalCode::SWB) {
                Some(("https://www.bcferries.com/current-conditions/SWB-SGI", "https://www.bcferries.com/current-conditions/departures?terminalCode=SWB", "https://www.bcferries.com/current-conditions/service-notices#Vancouver%20Island%20-%20Southern%20Gulf%20Islands"))
            } else if schedule.terminal_pair.includes_terminal(TerminalCode::TSA) {
                Some(("https://www.bcferries.com/current-conditions/TSA-SGI", "https://www.bcferries.com/current-conditions/departures?terminalCode=TSA", "https://www.bcferries.com/current-conditions/service-notices#Metro%20Vancouver%20-%20Southern%20Gulf%20Islands"))
            } else {
                None
            }
        }).unwrap_or(("https://www.bcferries.com/current-conditions", "https://www.bcferries.com/current-conditions/departures", "https://www.bcferries.com/current-conditions/service-notices"));
        html! { <>
            <div class="row mt-4">
                <div class="col-12 col-md-8 col-lg-6">
                    { self.sailings_html() }
                </div>
            </div>
            { if self.terminal_pair.includes_terminal(TerminalCode::TSA) { html! { <>
                <div class="mt-3">
                    <small>
                        <span class="text-nowrap">
                            <a href="https://www.bcferries.com/" target="_blank">{ "Reservations" }</a>
                            { " are recommended for direct sailings." }
                        </span>
                        { " " }
                        <span class="text-nowrap">
                            { "See here for more " }
                            <a href="https://www.bcferries.com/routes-fares/ferry-fares/thru-fare" target="_blank">{ "information about thru-fares" }</a>
                            { "." }
                        </span>
                    </small>
                </div>
            </> }} else {
                html! {}
            }}
            <div class="mt-3 text-muted">
                <small>
                    <div><strong>{ "BC Ferries may adjust schedules at any time and without notice." }</strong></div>
                    <div>
                        { "Confirm all sailings with the " }
                        <a class="link-secondary" href={ source_url } target="_blank">
                            { "original schedule" }
                        </a>
                        { ", and check " }
                        <a class="link-secondary" href={ service_notices_url } target="_blank">
                            { "service notices" }
                        </a>
                        { ", " }
                        <a class="link-secondary" href={ departures_url } target="_blank">
                            { "departures" }
                        </a>
                        { " and " }
                        <a class="link-secondary" href={ sailing_status_url } target="_blank">
                            { "sailing status" }
                        </a>
                        { " before you depart." }
                        { " If you find a mistake, send feedback to " }
                        <a class="link-secondary" href="mailto:ferries@borsboom.io" target="_blank">{ "ferries@borsboom.io" }</a>
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
        let terminal_pair = self.terminal_pair;
        let today = self.today;
        Callback::once(move |e: Event| {
            let orig_date_input = e.target_unchecked_into::<HtmlInputElement>().value();
            let trimmed_date_input = orig_date_input.trim();
            if trimmed_date_input.is_empty() {
                date_input_state.set(DateInputState { input: format_iso8601_date(today), value: Ok(today) });
                history
                    .push_with_query(
                        Route::Sailings,
                        SailingsQuery { from: Some(terminal_pair.from), to: Some(terminal_pair.to), date: None },
                    )
                    .unwrap();
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
                            SailingsQuery {
                                from: Some(terminal_pair.from),
                                to: Some(terminal_pair.to),
                                date: Some(date),
                            },
                        )
                        .unwrap();
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
        let terminal_pair = self.terminal_pair;
        let today = self.today;
        let new_date = opt_new_date.unwrap_or(today);
        Callback::once(move |_| {
            date_input_state.set(DateInputState { input: format_iso8601_date(new_date), value: Ok(new_date) });
            history
                .push_with_query(
                    Route::Sailings,
                    SailingsQuery { from: Some(terminal_pair.from), to: Some(terminal_pair.to), date: opt_new_date },
                )
                .unwrap();
        })
    }

    fn onclick_swap_terminals_button_callback(&self) -> Callback<MouseEvent> {
        let history = self.history.clone();
        let terminal_pair = self.terminal_pair;
        let query_date = self.query_date;
        Callback::once(move |_| {
            history
                .push_with_query(
                    Route::Sailings,
                    SailingsQuery { from: Some(terminal_pair.to), to: Some(terminal_pair.from), date: query_date },
                )
                .unwrap();
        })
    }

    fn html(self) -> Html {
        html! {
            <div class="d-print-none">
                <div class="row mb-1">
                    <label class="col-2 col-md-1 col-form-label">{ "From" }</label>
                    <div class="col-10 col-md-7 col-lg-5">
                        <span class="form-control">
                            { location_terminal_link_html(
                                self.terminal_pair.from,
                                SailingsQuery{ from: None, to: Some(self.terminal_pair.to), date: self.query_date }
                            ) }
                        </span>
                    </div>
                </div>
                <div class="row mb-1">
                    <label class="col-2 col-md-1 col-form-label">{ "To" }</label>
                    <div class="col-10 col-md-7 col-lg-5">
                        <span class="form-control">
                            { location_terminal_link_html(
                                self.terminal_pair.to,
                                SailingsQuery{ from: Some(self.terminal_pair.from), to: None, date: self.query_date }
                            ) }
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
                            class={ classes!("form-control", "align-self-center", "date-input", self.date_input_state.value.is_err().then(|| "is-invalid")) }
                            value={ self.date_input_state.input.to_owned() }
                            min={ format_iso8601_date(self.today) }
                            max={ format_iso8601_date(self.max_date) }
                            onchange={ self.onchange_date_input_callback() }/>
                        <button
                            type="button"
                            class="btn btn-outline-secondary border-0 pe-0"
                            title="Next Date"
                            onclick={ self.onclick_adjust_date_button_callback(Some(max(self.view_date.previous_day().unwrap(), self.today))) }
                            disabled={ self.date_input_state.value.as_ref().map(|d| *d <= self.today).unwrap_or(true) }
                        >
                            <i class="bi bi-caret-left-fill"/>
                        </button>
                        <button
                            type="button"
                            class="btn btn-outline-secondary border-0 ps-0"
                            title="Previous Date"
                            onclick={ self.onclick_adjust_date_button_callback(Some(min(self.view_date.next_day().unwrap(), self.max_date))) }
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
    let terminal_pair = TerminalCodePair { from: props.terminal_pair.from, to: props.terminal_pair.to };
    let query_date = props.date;
    let today = today_vancouver();
    let query_date_or_today = match query_date {
        None => today,
        Some(date) if date < today => today,
        Some(date) => date,
    };
    let history = use_history().unwrap();
    let schedules_state = use_context::<SchedulesState>().unwrap();
    let date_input_state = use_state(|| DateInputState {
        input: format_iso8601_date(query_date_or_today),
        value: Ok(query_date_or_today),
    });
    let sailings_model = SailingsModel::new(&schedules_state, &date_input_state, terminal_pair, query_date_or_today);
    let form_model = FormModel {
        history,
        date_input_state,
        terminal_pair,
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
