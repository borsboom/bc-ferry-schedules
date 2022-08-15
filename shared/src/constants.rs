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

pub static ROUTE5_GULF_ISLAND_TERMINALS: Lazy<HashSet<Terminal>> =
    Lazy::new(|| HashSet::from_iter([Terminal::PLH, Terminal::POB, Terminal::PSB, Terminal::PST, Terminal::PVB]));

pub static ROUTE5_TERMINAL_PAIRS: Lazy<HashSet<TerminalPair>> = Lazy::new(|| {
    HashSet::from_iter(
        ROUTE5_GULF_ISLAND_TERMINALS
            .iter()
            .flat_map(|&from| {
                ROUTE5_GULF_ISLAND_TERMINALS
                    .iter()
                    .filter(move |&to| from != *to)
                    .chain(iter::once(&Terminal::SWB))
                    .map(move |&to| TerminalPair { from, to })
            })
            .flat_map(|tp| [tp, tp.swapped()])
            .filter(|&tp| tp != TerminalPair { from: Terminal::SWB, to: Terminal::PLH }),
    )
});

pub static OTHER_ROUTES_TERMINAL_PAIRS: Lazy<HashSet<TerminalPair>> = Lazy::new(|| {
    HashSet::from_iter(
        [
            TerminalPair { from: Terminal::CHM, to: Terminal::PEN },
            TerminalPair { from: Terminal::CHM, to: Terminal::THT },
            TerminalPair { from: Terminal::FUL, to: Terminal::SWB },
            TerminalPair { from: Terminal::PEN, to: Terminal::THT },
            TerminalPair { from: Terminal::PLH, to: Terminal::TSA },
            TerminalPair { from: Terminal::POB, to: Terminal::TSA },
            TerminalPair { from: Terminal::PSB, to: Terminal::TSA },
            TerminalPair { from: Terminal::PST, to: Terminal::TSA },
            TerminalPair { from: Terminal::PVB, to: Terminal::TSA },
            TerminalPair { from: Terminal::SWB, to: Terminal::TSA },
            TerminalPair { from: Terminal::VES, to: Terminal::CFT },
        ]
        .iter()
        .flat_map(|&tp| [tp, tp.swapped()]),
    )
});

pub static ALL_TERMINAL_PAIRS: Lazy<HashSet<TerminalPair>> =
    Lazy::new(|| HashSet::from_iter(ROUTE5_TERMINAL_PAIRS.union(&*OTHER_ROUTES_TERMINAL_PAIRS).cloned()));

pub static ALL_AREA_PAIRS: Lazy<HashSet<AreaPair>> =
    Lazy::new(|| HashSet::from_iter(ALL_TERMINAL_PAIRS.iter().map(|tp| tp.area_pair())));

pub static AREA_TERMINALS: Lazy<HashMap<Area, HashSet<Terminal>>> =
    Lazy::new(|| into_hashset_group_map(Terminal::iter(), |t| t.area()));

pub static AREA_PAIR_TERMINAL_PAIRS: Lazy<HashMap<AreaPair, HashSet<TerminalPair>>> =
    Lazy::new(|| into_hashset_group_map(ALL_TERMINAL_PAIRS.iter().cloned(), |tp| tp.area_pair()));
