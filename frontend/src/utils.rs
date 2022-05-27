use crate::imports::*;
use crate::types::*;

pub fn format_time(time: Time) -> String {
    time.format(format_description!("[hour repr:12 padding:none]:[minute] [period case:lower]")).unwrap()
}

pub fn human_time(time: OffsetDateTime) -> HumanTime {
    // Using duration because default way of getting system time doesn't work on browser WASM
    HumanTime::from_seconds((time - now_vancouver()).whole_seconds())
}

pub fn location_terminal_link_html(terminal: TerminalCode, query: SailingsQuery) -> Html {
    html! { <>
        <strong>
            <Link<Route, SailingsQuery> to={Route::Sailings} {query}>{ terminal.long_location_name() }</Link<Route, SailingsQuery>>
        </strong>
        <small class="text-muted">
            { " ("}
            { terminal.terminal_name() }
            { ")"}
        </small>
    </> }
}

pub fn location_terminal_html(terminal: TerminalCode) -> Html {
    html! { <>
        <strong>
            { terminal.long_location_name() }
        </strong>
        <span class="text-muted font-weight-normal">
            { " ("}
            { terminal.terminal_name() }
            { ")"}
        </span>
    </> }
}
