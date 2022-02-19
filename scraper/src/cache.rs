use crate::imports::*;
use crate::types::*;
use crate::utils::*;
use ::directories::ProjectDirs;
use ::scraper::Html;
use ::std::fs;
use ::std::path::PathBuf;

static MAX_CACHE_AGE: Lazy<Duration> = Lazy::new(|| Duration::hours(12));

#[derive(Debug)]
pub struct Cached<T> {
    pub value: T,
    pub changed: bool,
}

#[derive(Debug)]
pub struct Cache<'a> {
    options: &'a Options,
    project_dirs: &'a ProjectDirs,
    reqwest_client: reqwest::Client,
}

impl<'a> Cache<'a> {
    pub fn new(options: &'a Options, project_dirs: &'a ProjectDirs) -> Cache<'a> {
        let reqwest_client = reqwest::Client::new();
        Cache { options, project_dirs, reqwest_client }
    }

    pub async fn fetch_url<T, F>(&self, url: &str, transform: F) -> Result<Cached<T>>
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
                if !self.options.ignore_cache {
                    let cache_modified_time: DateTime<Utc> = cache_metadata.modified()?.into();
                    if Utc::now() - cache_modified_time < *MAX_CACHE_AGE {
                        info!("Using cached: {:?}", cache_path);
                        return Ok(Cached { value: cached_value, changed: self.options.force });
                    }
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
                Ok(Cached { value: new_value, changed: true })
            } else {
                Ok(Cached { value: new_value, changed: self.options.force }) as Result<_>
            }
        };
        inner.await.with_context(|| format!("Failed to fetch URL with cache: {:?}", url))
    }

    pub async fn get_html(&self, url: &str, ignore_changes_regex: &Regex) -> Result<Cached<Html>> {
        let transform_html = |orig_contents: String| {
            let contents = ignore_changes_regex.replace(&orig_contents, "").to_string();
            (Html::parse_document(&contents), contents)
        };
        self.fetch_url(url, transform_html).await
    }
}
