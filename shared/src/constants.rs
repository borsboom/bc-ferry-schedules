use crate::imports::*;
use crate::types::*;
use crate::utils::*;

pub const BCFERRIES_BASE_URL: &str = "https://www.bcferries.com";
pub const BCFERRIES_HOME_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/");
pub const ALL_SCHEDULES_URL: &str = concatcp!(BCFERRIES_BASE_URL, "/routes-fares/schedules");
pub const ROUTE5_SCHEDULES_URL: &str = concatcp!(ALL_SCHEDULES_URL, "/southern-gulf-islands");
pub const OTHER_ROUTE_SCHEDULES_BASE_URL: &str = concatcp!(ALL_SCHEDULES_URL, "/seasonal");
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

pub static ROUTE_5_AND_9_GULF_ISLAND_TERMINALS: Lazy<HashSet<Terminal>> =
    Lazy::new(|| HashSet::from_iter([Terminal::PLH, Terminal::POB, Terminal::PSB, Terminal::PST, Terminal::PVB]));

pub static ALL_TERMINAL_PAIRS: Lazy<HashSet<TerminalPair>> = Lazy::new(|| {
    let routes = [
        // Route 1 (Tsawwassen/Swartz Bay)
        vec![Terminal::TSA, Terminal::SWB],
        // Route 4 (Fulford Harbour/Swartz Bay)
        vec![Terminal::FUL, Terminal::SWB],
        // Route 5 (Swartz Bay/Southern Gulf Islands)
        vec![Terminal::SWB, Terminal::PLH, Terminal::POB, Terminal::PSB, Terminal::PST, Terminal::PVB],
        // Route 6 (Vesuvius/Crofton)
        vec![Terminal::VES, Terminal::CFT],
        // Route 9 (Tsawwassen/Southern Gulf Islands)
        vec![Terminal::TSA, Terminal::PLH, Terminal::POB, Terminal::PSB, Terminal::PST, Terminal::PVB],
        // Route 12 (Brentwood/Mill Bay)
        vec![Terminal::BTW, Terminal::MIL],
        // Route 20 (Chemainus/Thetis/Penelakut)
        // TEMPORARILY DISABLED: vec![Terminal::CHM, Terminal::THT, Terminal::PEN],
    ];
    routes.iter().flat_map(|terminals| Terminal::combinations(terminals)).collect()
});

pub static ALL_AREA_PAIRS: Lazy<HashSet<AreaPair>> =
    Lazy::new(|| HashSet::from_iter(ALL_TERMINAL_PAIRS.iter().map(|tp| tp.area_pair())));

pub static AREA_TERMINALS: Lazy<HashMap<Area, HashSet<Terminal>>> =
    Lazy::new(|| into_hashset_group_map(Terminal::iter(), |t| t.area()));

pub static AREA_PAIR_TERMINAL_PAIRS: Lazy<HashMap<AreaPair, HashSet<TerminalPair>>> =
    Lazy::new(|| into_hashset_group_map(ALL_TERMINAL_PAIRS.iter().cloned(), |tp| tp.area_pair()));
