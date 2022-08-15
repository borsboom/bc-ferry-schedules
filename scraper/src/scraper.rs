mod other_routes;
mod route5;

use crate::cache::*;
use crate::imports::*;
use crate::scraper::other_routes::*;
use crate::scraper::route5::*;
use crate::types::*;

pub async fn scrape_schedules(options: &Options, cache: &Cache<'_>) -> Result<Vec<Schedule>> {
    let inner = async {
        let today = today_vancouver();
        let mut result = Vec::new();
        result.extend(scrape_route5_schedules(options, cache, today).await?);
        for &terminal_pair in OTHER_ROUTES_TERMINAL_PAIRS.iter() {
            result.extend(scrape_other_route_schedules(options, cache, terminal_pair, today).await?);
        }
        Ok(result) as Result<_>
    };
    inner.await.context("Failed to scrape schedules")
}
