use directories::ProjectDirs;
use scraper::Html;
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic, Arc};
use tokio_retry::{strategy, Retry};

use crate::imports::*;
use crate::macros::*;
use crate::utils::*;

const MAX_RETRIES: usize = 5;

#[derive(Debug)]
pub struct Cache<'a> {
    max_cache_age: Duration,
    project_dirs: &'a ProjectDirs,
    reqwest_client: reqwest::Client,
}

impl<'a> Cache<'a> {
    pub fn new(max_cache_age: Duration, project_dirs: &'a ProjectDirs) -> Cache<'a> {
        let reqwest_client = reqwest::Client::new();
        Cache { max_cache_age, project_dirs, reqwest_client }
    }

    async fn fetch_retry_action<T, F>(
        &self,
        url: &str,
        retry_number: Arc<AtomicUsize>,
        transform: &F,
    ) -> Result<(T, String)>
    where
        F: Fn(String) -> Result<(T, String)>,
    {
        let retry_number = retry_number.fetch_add(1, atomic::Ordering::SeqCst) + 1;
        let inner = async {
            let response = self.reqwest_client.get(url).send().await?.error_for_status()?;
            transform(response.text().await?)
        };
        let result = inner.await;
        match &result {
            Err(err) if retry_number <= MAX_RETRIES => {
                warn!("Will retry (#{} of {}) fetching {:?} due to: {}", retry_number, MAX_RETRIES, url, err)
            }
            _ => {}
        }
        result
    }

    pub async fn fetch_url<T, F>(&self, url: &str, transform: F) -> Result<T>
    where
        F: Fn(String) -> Result<(T, String)>,
    {
        let inner = async {
            let mut cache_path = PathBuf::new();
            cache_path.push(self.project_dirs.cache_dir());
            let cache_filename = format!("{}_{}", regex!(r"[^\w\d-]+").replace_all(url, "_"), calculate_hash(&url));
            cache_path.push(&cache_filename);
            if let Ok(cache_metadata) = fs::metadata(&cache_path) {
                let cache_modified_time: OffsetDateTime = cache_metadata.modified()?.into();
                if OffsetDateTime::now_utc() - cache_modified_time < self.max_cache_age {
                    if let Ok((cached_value, _)) = transform(fs::read_to_string(&cache_path)?) {
                        info!("Using cached: {:?}", cache_path);
                        return Ok(cached_value);
                    }
                }
            }
            info!("Fetching: {:?}", url);
            let retry_number = Arc::new(AtomicUsize::new(0));
            let (value, contents): (_, String) =
                Retry::spawn(strategy::FibonacciBackoff::from_millis(5).factor(1000).take(MAX_RETRIES), || {
                    self.fetch_retry_action(url, retry_number.clone(), &transform)
                })
                .await?;
            fs::create_dir_all(self.project_dirs.cache_dir())?;
            fs::write(&cache_path, &contents)?;
            Ok(value) as Result<_>
        };
        inner.await.with_context(|| format!("Failed to fetch URL with cache: {:?}", url))
    }

    pub async fn get_html(&self, url: &str, error_regex: &Regex) -> Result<Html> {
        let transform_html = |contents: String| {
            if error_regex.is_match(&contents) {
                bail!("HTML contains error text")
            } else {
                let doc = Html::parse_document(&contents);
                let html = doc.root_element().html();
                Ok((doc, html))
            }
        };
        self.fetch_url(url, transform_html).await
    }
}
