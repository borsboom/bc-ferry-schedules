mod imports;
mod sailings_component;
mod sailings_processor;
mod types;
mod utils;

use reqwasm::http;

use crate::imports::*;
use crate::sailings_component::*;
use crate::types::*;
use crate::utils::*;

#[function_component(Navbar)]
fn navbar_component() -> Html {
    let route: Route = use_route().unwrap_or_default();
    html! {
        <nav class="mb-3 navbar navbar-expand navbar-dark rounded d-print-none bg-heading">
            <div class="container-fluid">
                <Link<Route> classes="navbar-brand" to={Route::Home}>
                    <img src="/assets/logo.png" width="30" height="30" alt="B.C. Ferry Schedules"/>
                </Link<Route>>
                <div class="collapse navbar-collapse">
                    <ul class="navbar-nav">
                        <li class="nav-item">
                            <Link<Route> classes={classes!("nav-link", matches!(route, Route::Home).then(|| "active"))} to={Route::Home}>
                                { "Home" }
                            </Link<Route>>
                        </li>
                    </ul>
                </div>
                <ul class="navbar-nav">
                    <li class="nav-item">
                        <a title="Ko-fi" class="btn btn-outline-light btn-sm" href="https://ko-fi.com/borsboom" target="#blank">
                            <img src="/assets/ko-fi.png" height="18"/>
                            <small>{ " Buy me a coffee" }</small>
                        </a>
                    </li>
                </ul>
            </div>
        </nav>
    }
}

fn select_from_terminal_html(query: &SailingsQuery) -> Html {
    html! { <>
        <p class="mt-3">
            { if query.to.is_none() {
                "To get started, select your departure terminal:"
            } else {
                "Select your departure terminal:"
            }}
        </p>
        <ul>
            { for TerminalCode::iter().map(|from| html!{
                <li>
                    { location_terminal_link_html(from, SailingsQuery{from: Some(from), ..*query}) }
                </li>
            })}
        </ul>
    </> }
}

fn select_to_terminal_html(query: &SailingsQuery) -> Html {
    let terminal_html = |to| {
        if query.from.map(|from| TerminalCodePair { from, to }.is_visible()).unwrap_or(true) {
            html! {
                <li>{ location_terminal_link_html(to, SailingsQuery{to: Some(to), ..*query}) }</li>
            }
        } else {
            html! {}
        }
    };
    html! { <>
        <p class="mt-3">{ "Select your arrival terminal:" }</p>
        <ul>
            { for TerminalCode::iter().map(terminal_html) }
        </ul>
    </> }
}

fn home_html() -> Html {
    html! { <>
        <h1 class="display-6">
            { "B.C. Ferry Schedules" }
            <small class="text-muted">{ " for the Outer Gulf Islands" }</small>
        </h1>
        <p class="lead">
            { "An easy to use and understand presentation of the BC Ferries schedules for routes five and nine, serving Galiano, Mayne, Pender and Saturna Islands and Long Harbour (on Salt Spring Island) to/from Victoria and Vancouver. Just select your terminals and date, and you're shown the sailings for that day."}
        </p>
        { select_from_terminal_html(&SailingsQuery::new()) }
        <p>
            { "These routes are among the most complex and confusing in the system, and even the most seasoned ferry user is prone to mis-reading the original schedules." }
        </p>
        <div class="p-2 bg-light border rounded">
            <div><strong>{ "Do not rely on this site as your only source of schedule information!" }</strong></div>
            <div>
                { "The schedule data is scraped from BC Ferries' web site and then processed into individual sailings. "}
                { "This is error prone and the data may be out of date or incorrect. Be sure to double check against the " }
                <a class="link-dark" href="https://www.bcferries.com/routes-fares/schedules" target="#blank">{ "official schedules" }</a>
                { "." }
            </div>
        </div>
    </> }
}

#[function_component(SailingsPage)]
fn sailings_page_component() -> Html {
    let location = use_location();
    let query = location
        .and_then(|l| l.query().map_err(|e| error!("Invalid sailings query: {}", e)).ok())
        .unwrap_or_else(SailingsQuery::new);
    html! { <>
        <h1 class="display-6 mb-3 small">
            { "B.C. Ferry Schedule" }
        </h1>
        <h5 class={ if query.from.is_some() && query.to.is_some() { "d-none d-print-block" } else { "" } }>
            { if let Some(from) = query.from { html! {
                <div>
                    { "From "}
                    { location_terminal_html(from) }
                </div>
            }} else {
                html! {}
            }}
            { match query.to {
                Some(to) if query.from.map(|from| (TerminalCodePair{ from, to }).is_visible()).unwrap_or(true) => html! {
                    <div>
                        { "To " }
                        { location_terminal_html(to) }
                    </div>
                },
                _ => html! {},
            }}
        </h5>
        { match query {
            SailingsQuery { from: None, .. } => select_from_terminal_html(&query),
            SailingsQuery { from: Some(_), to: None, .. } => select_to_terminal_html(&query),
            SailingsQuery { from: Some(from), to: Some(to), date } => {
                if (TerminalCodePair { from, to }.is_visible()) { html! {
                    <Sailings terminal_pair={TerminalCodePair{from, to}} {date}/>
                }} else {
                    select_to_terminal_html(&query)
                }
            }
        }}
    </> }
}

fn not_found_html() -> Html {
    html! { <>
        <h1>{ "Lost at sea (page not found)" }</h1>
        <p>
            <Link<Route> to={Route::Home}>{ "Activate rescue beacon" }</Link<Route>>
            { " (go to home page)" }
        </p>
    </> }
}

fn switch_route(route: &Route) -> Html {
    match route {
        Route::Home => home_html(),
        Route::Sailings => html! { <SailingsPage/> },
        Route::NotFound => not_found_html(),
    }
}

fn footer_html() -> Html {
    html! {
        <div class="small">
            <hr class="mb-1"/>
            <div>
                <a href="https://github.com/borsboom/bc-ferry-schedules/actions/workflows/scrape.yaml" target="#blank">
                    <img src="https://github.com/borsboom/bc-ferry-schedules/actions/workflows/scrape.yaml/badge.svg" alt="scrape status"/>
                </a>
            </div>
            <div>
                { "Created by " }
                <a class="link-dark" href="https://borsboom.io/" target="#blank">{ "Emanuel Borsboom" }</a>
                { ". " }
                { "Source code on " }
                <a class="link-dark" href="https://github.com/borsboom/bc-ferry-schedules" target="#blank">{ "Github" }</a>
                { ". " }
                { "Send feedback to " }
                <a class="link-dark" href="mailto:ferries@borsboom.io" target="#blank">{ "ferries@borsboom.io" }</a>
                { "." }
            </div>
            <div class="text-muted">{ "This site is independently operated and is not affiliated with British Columbia Ferry Services Inc." }</div>
        </div>
    }
}

async fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let response = http::Request::get(url).send().await?;
    let json = response.json::<T>().await?;
    Ok(json)
}

fn load_schedules_state(schedules_state: UseStateHandle<SchedulesState>) {
    schedules_state.set(SchedulesState::Loading);
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_json::<Vec<Schedule>>("/v1/schedules.json").await {
            Ok(schedules) => {
                let terminal_pair_schedules_map = into_group_map(schedules, |s| (s.terminal_pair, s));
                schedules_state.set(SchedulesState::Loaded(Rc::new(terminal_pair_schedules_map)));
            }
            Err(err) => {
                error!("{}", err);
                schedules_state.set(SchedulesState::Failed);
            }
        }
    });
}

#[function_component(App)]
fn app() -> Html {
    let schedules_state = use_state(|| SchedulesState::Init);
    if let SchedulesState::Init = *schedules_state {
        load_schedules_state(schedules_state.clone());
    }
    html! {
        <ContextProvider<SchedulesState> context={(*schedules_state).clone()}>
            <BrowserRouter>
                <div class="container">
                    <Navbar/>
                    <Switch<Route> render={Switch::render(switch_route)}/>
                    { footer_html() }
                </div>
            </BrowserRouter>
        </ContextProvider<SchedulesState>>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
