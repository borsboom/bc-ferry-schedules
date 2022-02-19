mod non_tsawwassen;
mod tsawwassen;

use crate::cache::*;
use crate::imports::*;
use crate::scraper::non_tsawwassen::*;
use crate::scraper::tsawwassen::*;
use crate::types::*;
use crate::utils::*;

pub async fn scrape_schedules(options: &Options, cache: &Cache<'_>) -> Result<Vec<Schedule>> {
    let inner = async {
        let today = now_pacific().date().naive_local();
        let mut result = Vec::new();
        result.extend(scrape_non_tsawwassen_schedules(options, cache, today).await?);
        for terminal_pair in
            [TerminalCode::Psb, TerminalCode::Pvb, TerminalCode::Pob, TerminalCode::Plh, TerminalCode::Pst]
                .iter()
                .flat_map(|&t| {
                    [
                        TerminalCodePair { from: t, to: TerminalCode::Tsa },
                        TerminalCodePair { from: TerminalCode::Tsa, to: t },
                    ]
                })
        {
            result.extend(scrape_tsawwassen_schedules(options, cache, terminal_pair, today).await?);
        }
        Ok(result) as Result<_>
    };
    inner.await.context("Failed to scrape schedules")
}
