mod annotations;
mod cache;
mod constants;
mod depart_time_and_row_annotations;
mod dynamodb;
mod imports;
mod sailings;
mod scraper;
mod types;
mod utils;
mod weekday_dates;

use crate::cache::Cache;
use crate::dynamodb::put_dynamodb;
use crate::imports::*;
use crate::scraper::scrape_schedules;
use crate::types::Options;
use ::clap::Parser;
use ::directories::ProjectDirs;
use ::std::env;
use ::std::process;

#[derive(Parser, Debug)]
pub struct CliArgs {
    /// Logging verbosity level (valid values: off, error, warn, info, debug, trace)
    #[clap(short, long, value_name = "LEVEL", default_value = "info")]
    verbosity: log::LevelFilter,

    /// Don't write processed data to DynamoDB
    #[clap(short = 'n', long)]
    pub dry_run: bool,

    #[clap(flatten)]
    options: Options,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let cli_args = CliArgs::parse();
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_ok() {
        env_logger::init();
    } else {
        env_logger::builder()
            .filter(Some(env!("CARGO_PKG_NAME")), cli_args.verbosity)
            .format_timestamp(None)
            .format_target(false)
            .init();
    }
    let inner = async {
        let project_dirs = ProjectDirs::from("io", "borsboom", env!("CARGO_PKG_NAME"))
            .ok_or_else(|| anyhow!("Could not get project directories"))?;
        let cache = Cache::new(&cli_args.options, &project_dirs);
        let schedules = scrape_schedules(&cli_args.options, &cache).await?;
        if !cli_args.dry_run {
            put_dynamodb(&cli_args.options, &schedules).await?;
        }
        Ok(()) as Result<()>
    };
    if let Err(error) = inner.await {
        error!("{:?}", error);
        process::exit(1);
    }
}
