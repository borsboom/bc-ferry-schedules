use ::anyhow::{anyhow, Result};
use ::clap::Parser;
use ::directories::ProjectDirs;
use ::std::env;
use ferrysched_scraper::*;

#[derive(Parser, Debug)]
pub struct CliArgs {
    /// Logging verbosity level
    #[clap(short, long, default_value = "info")]
    verbosity: log::LevelFilter,

    /// Write processed data to DynamoDB
    #[clap(short, long)]
    pub put_dynamodb: bool,

    #[clap(flatten)]
    options: Options,
}

#[tokio::main]
async fn main() -> Result<()> {
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
    let project_dirs = ProjectDirs::from("io", "borsboom", env!("CARGO_PKG_NAME"))
        .ok_or_else(|| anyhow!("Could not get project directories"))?;
    let cache = Cache::new(&cli_args.options, &project_dirs);
    let schedules = scrape_non_tsawwassen_schedules(&cache).await?;
    if cli_args.put_dynamodb {
        put_dynamodb(&schedules).await?;
    }
    Ok(())
}
