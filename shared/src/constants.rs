use crate::imports::*;

pub const BCFERRIES_BASE_URL: &str = "https://www.bcferries.com";
pub const BCFERRIES_HOME_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/");
pub const ALL_SCHEDULES_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/routes-fares/schedules");
pub const SGI_SCHEDULES_URL: &str = concatcp!(ALL_SCHEDULES_URL, "/southern-gulf-islands");
pub const SEASONAL_SCHEDULES_BASE_URL: &str = concatcp!(ALL_SCHEDULES_URL, "/seasonal");
pub const ALL_SAILING_STATUS_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/current-conditions");
pub const SWB_SGI_SAILING_STATUS_URL: &str = concatcp!(ALL_SAILING_STATUS_URL, "/SWB-SGI");
pub const TSA_SGI_SAILING_STATUS_URL: &str = concatcp!(ALL_SAILING_STATUS_URL, "/TSA-SGI");
pub const ALL_DEPARTURES_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/current-conditions/departures");
pub const SWB_DEPARTURES_URL: &str = concatcp!(ALL_DEPARTURES_URL, "?terminalCode=SWB");
pub const TSA_DEPARTURES_URL: &str = concatcp!(ALL_DEPARTURES_URL, "?terminalCode=TSA");
pub const ALL_SERVICE_NOTICES_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/current-conditions/service-notices");
pub const SWB_SGI_SERVICE_NOTICES_URL: &str =
    concatcp!(ALL_SERVICE_NOTICES_URL, "#Vancouver%20Island%20-%20Southern%20Gulf%20Islands");
pub const TSA_SGI_SERVICE_NOTICES_URL: &str =
    concatcp!(ALL_SERVICE_NOTICES_URL, "#Metro%20Vancouver%20-%20Southern%20Gulf%20Islands");
pub const THRU_FARE_INFORMATION_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/routes-fares/ferry-fares/thru-fare");
