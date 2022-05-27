use clap::Args;

use crate::imports::*;

#[derive(Args, Debug)]
pub struct Options {
    /// Maximum time to cache source schedule HTML
    #[clap(short = 'a', long, value_name = "HOURS", default_value = "12")]
    pub max_cache_age: i64,

    /// Only process schedules for specified terminal pair
    #[clap(short, long, value_name = "FROM-TO")]
    pub terminals: Option<TerminalCodePair>,

    /// Only process schedules whose date range includes this date
    #[clap(short, long, value_name = "YYYY-MM-DD", parse(try_from_str = parse_iso8601_date))]
    pub date: Option<Date>,

    /// Write output schedules JSON to this file
    #[clap(short, long, value_name = "PATH")]
    pub output_file: Option<PathBuf>,

    /// Upload schedules JSON to this S3 bucket
    #[clap(short = 'b', long, value_name = "NAME")]
    pub output_s3_bucket: Option<String>,

    /// Upload schedules JSON to this S3 key
    #[clap(short = 'k', long, value_name = "KEY", default_value = "schedules.json")]
    pub output_s3_key: String,

    /// After uploading schedules JSON, invalidate this CloudFront distribution
    #[clap(short = 'c', long, value_name = "DISTRIBUTION ID")]
    pub invalidate_cloudfront_distribution_id: Option<String>,
}
