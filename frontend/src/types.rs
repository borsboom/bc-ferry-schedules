use crate::imports::*;

#[derive(Clone, Eq, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/sailings")]
    Sailings,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct SailingsQuery {
    pub from: Option<Area>,
    pub to: Option<Area>,
    pub date: Option<Date>,
}

impl SailingsQuery {
    pub fn new() -> SailingsQuery {
        SailingsQuery { from: None, to: None, date: None }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, SailingsQuery { from: None, to: None, date: None })
    }
}

#[derive(Clone)]
pub enum SchedulesState {
    Init,
    Loading,
    Loaded(Rc<HashMap<TerminalPair, Vec<Schedule>>>),
    Failed,
}

impl PartialEq for SchedulesState {
    fn eq(&self, other: &SchedulesState) -> bool {
        // For efficiency, we don't compare the contents because schedules are only loaded once
        matches!(
            (self, other),
            (SchedulesState::Init, SchedulesState::Init)
                | (SchedulesState::Loading, SchedulesState::Loading)
                | (SchedulesState::Loaded(_), SchedulesState::Loaded(_))
                | (SchedulesState::Failed, SchedulesState::Failed)
        )
    }
}
