use crate::imports::*;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/sailings")]
    Sailings,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct SailingsQuery {
    pub from: Option<TerminalCode>,
    pub to: Option<TerminalCode>,
    pub date: Option<NaiveDate>,
}

impl SailingsQuery {
    pub fn new() -> SailingsQuery {
        SailingsQuery { from: None, to: None, date: None }
    }
}

#[derive(Clone)]
pub enum SchedulesState {
    Init,
    Loading,
    Loaded(Rc<HashMap<TerminalCodePair, Vec<Schedule>>>),
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
