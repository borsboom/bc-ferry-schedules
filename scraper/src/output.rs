use crate::imports::*;
use crate::types::*;

use tempfile::NamedTempFile;

static S3_CACHE_MAX_AGE: Lazy<Duration> = Lazy::new(|| Duration::hours(12));

async fn upload_to_s3(
    aws_config: &aws_types::SdkConfig,
    bucket: &str,
    key: &str,
    schedules: &[Schedule],
) -> Result<()> {
    info!("Uploading schedules JSON to: s3://{}/{}", bucket, key);
    let s3_client = aws_sdk_s3::Client::new(aws_config);
    s3_client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type("application/json")
        .acl(aws_sdk_s3::model::ObjectCannedAcl::PublicRead)
        .cache_control(format!("max-age={},public", S3_CACHE_MAX_AGE.whole_seconds()))
        .body(aws_sdk_s3::types::ByteStream::from(
            serde_json::to_vec(schedules).expect("schedules to serialize to JSON"),
        ))
        .send()
        .await
        .with_context(|| format!("Failed to upload schedules JSON to S3: s3://{}/{}", bucket, key))?;
    Ok(())
}

async fn invalidate_cloudfront_distribution(
    aws_config: &aws_types::SdkConfig,
    distribution_id: &str,
    s3_key: &str,
) -> Result<()> {
    let cloudfront_client = aws_sdk_cloudfront::Client::new(aws_config);
    let path = format!("/{}", s3_key);
    info!("Invalidating CloudFront distribution {:?} for path: {:?}", distribution_id, path);
    cloudfront_client
        .create_invalidation()
        .distribution_id(distribution_id)
        .invalidation_batch(
            aws_sdk_cloudfront::model::InvalidationBatch::builder()
                .caller_reference(OffsetDateTime::now_utc().unix_timestamp_nanos().to_string())
                .paths(aws_sdk_cloudfront::model::Paths::builder().quantity(1).items(path).build())
                .build(),
        )
        .send()
        .await
        .with_context(|| {
            format!("Failed to create CloudFront invalidation for distribution ID: {:?}", distribution_id)
        })?;
    Ok(())
}

pub async fn write_output(options: &Options, schedules: &[Schedule]) -> Result<()> {
    let inner = async {
        if let (None, None) = (options.output_file.as_ref(), options.output_s3_bucket.as_ref()) {
            serde_json::to_writer_pretty(io::stdout(), &schedules)
                .context("Failed to write schedules JSON to standard output")?;
        } else {
            if let Some(output_file_path) = &options.output_file {
                info!("Writing schedules JSON to: {:?}", output_file_path);
                let mut temp_file = NamedTempFile::new_in(
                    output_file_path
                        .parent()
                        .unwrap_or(&std::env::current_dir().context("Failed to get current directory")?),
                )
                .with_context(|| format!("Failed to create schedules JSON output file: {:?}", output_file_path))?;
                serde_json::to_writer(&mut temp_file, &schedules)
                    .with_context(|| format!("Failed to write schedules JSON to file: {:?}", output_file_path))?;
                temp_file
                    .persist(output_file_path)
                    .context(format!("Failed to persist temporary file to: {:?}", output_file_path))?;
            }
            if let Some(bucket) = &options.output_s3_bucket {
                let aws_config = aws_config::from_env().load().await;
                upload_to_s3(&aws_config, bucket, &options.output_s3_key, schedules).await?;
                if let Some(distribution_id) = &options.invalidate_cloudfront_distribution_id {
                    invalidate_cloudfront_distribution(&aws_config, distribution_id, &options.output_s3_key).await?;
                }
            }
        }
        Ok(()) as Result<_>
    };
    inner.await.context("Failed to write output")
}
