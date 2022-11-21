use crate::imports::*;
use crate::types::*;

pub fn format_time(time: Time) -> String {
    time.format(format_description!("[hour repr:12 padding:none]:[minute] [period case:lower]"))
        .expect("Expect friendly time to format")
}

pub fn human_time(time: OffsetDateTime) -> HumanTime {
    // Using duration because default way of getting system time doesn't work on browser WASM
    HumanTime::from_seconds((time - now_vancouver()).whole_seconds())
}

pub fn area_link_html(area: Area, query: SailingsQuery) -> Html {
    html! {
        <Link<Route, SailingsQuery> to={Route::Sailings} {query}>{ area.long_name() }</Link<Route, SailingsQuery>>
    }
}
