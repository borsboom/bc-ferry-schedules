use crate::imports::*;
use crate::types::*;

pub fn format_time(time: NaiveTime) -> String {
    time.format("%l:%M %P").to_string()
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
