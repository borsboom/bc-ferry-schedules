use directories::ProjectDirs;
use scraper::Html;

use crate::imports::*;
use crate::macros::*;
use crate::utils::*;

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

    pub async fn fetch_url<T, F>(&self, url: &str, transform: F) -> Result<T>
    where
        F: Fn(String) -> (T, String),
    {
        let inner = async {
            let mut cache_path = PathBuf::new();
            cache_path.push(self.project_dirs.cache_dir());
            let cache_filename = format!("{}_{}", regex!(r"[^\w\d-]+").replace_all(url, "_"), calculate_hash(&url));
            cache_path.push(&cache_filename);
            let opt_cached_contents = if let Ok(cache_metadata) = fs::metadata(&cache_path) {
                let (cached_value, cached_contents) = transform(fs::read_to_string(&cache_path)?);
                let cache_modified_time: DateTime<Utc> = cache_metadata.modified()?.into();
                if Utc::now() - cache_modified_time < self.max_cache_age {
                    info!("Using cached: {:?}", cache_path);
                    return Ok(cached_value);
                }
                Some(cached_contents)
            } else {
                None
            };
            info!("Fetching: {:?}", url);
            let response = self.reqwest_client.get(url).send().await?.error_for_status()?;
            let (new_value, new_contents) = transform(response.text().await?);
            fs::create_dir_all(self.project_dirs.cache_dir())?;
            fs::write(&cache_path, &new_contents)?;
            if Some(&new_contents) != opt_cached_contents.as_ref() {
                cache_path.pop();
                cache_path.push("archive");
                fs::create_dir_all(&cache_path)?;
                cache_path.push(format!("{}_{}", cache_filename, now_pacific().format("%Y%m%d%H%M%S")));
                fs::write(&cache_path, &new_contents)?;
            }
            Ok(new_value) as Result<_>
        };
        inner.await.with_context(|| format!("Failed to fetch URL with cache: {:?}", url))
    }

    pub async fn get_html(&self, url: &str, ignore_changes_regex: &Regex) -> Result<Html> {
        let transform_html = |orig_contents: String| {
            let contents = ignore_changes_regex.replace(&orig_contents, "").to_string();
            (Html::parse_document(&contents), contents)
        };
        self.fetch_url(url, transform_html).await
    }
}
