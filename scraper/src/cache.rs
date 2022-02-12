use crate::imports::*;
use crate::types::*;
use crate::utils::*;
use ::directories::ProjectDirs;
use ::scraper::Html;
use ::std::fs;
use ::std::path::PathBuf;

static MAX_CACHE_AGE: Lazy<Duration> = Lazy::new(|| Duration::hours(1));

#[derive(Debug)]
pub enum Cached<T> {
    Unchanged,
    Contents(T),
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

    pub async fn get_url<T, F>(&self, url: &str, transform: F) -> Result<Cached<T>>
    where
        F: Fn(String) -> (T, String),
    {
        let mut cache_path = PathBuf::new();
        cache_path.push(self.project_dirs.cache_dir());
        let cache_filename = regex!(r"[^\w\d-]+").replace_all(url, "_");
        cache_path.push(cache_filename.as_ref());
        let opt_existing_contents = if let Ok(cache_metadata) = fs::metadata(&cache_path) {
            if !self.options.ignore_cache {
                info!("Checking cache: {:?}", cache_path);
                let cache_modified_time: DateTime<Utc> = cache_metadata.modified()?.into();
                if Utc::now() - cache_modified_time < *MAX_CACHE_AGE {
                    if self.options.force {
                        return Ok(Cached::Contents(transform(fs::read_to_string(&cache_path)?).0));
                    } else {
                        return Ok(Cached::Unchanged);
                    }
                }
            }
            Some(transform(fs::read_to_string(&cache_path)?).1)
        } else {
            None
        };
        info!("Fetching: {:?}", url);
        let (new_result, new_contents) = transform(self.reqwest_client.get(url).send().await?.text().await?);
        fs::create_dir_all(self.project_dirs.cache_dir())?;
        fs::write(&cache_path, &new_contents)?;
        if Some(&new_contents) != opt_existing_contents.as_ref() {
            cache_path.pop();
            cache_path.push("archive");
            fs::create_dir_all(&cache_path)?;
            cache_path.push(format!("{}_{}", cache_filename, now_pacific().format("%Y%m%d%H%M%S")));
            fs::write(&cache_path, &new_contents)?;
        } else if !self.options.force {
            return Ok(Cached::Unchanged);
        }
        Ok(Cached::Contents(new_result))
    }

    pub async fn get_html(&self, url: &str, ignore_changes_regex: &Regex) -> Result<Cached<Html>> {
        let transform_html = |orig_contents: String| {
            let contents = ignore_changes_regex.replace(&orig_contents, "").to_string();
            (Html::parse_document(&contents), contents)
        };
        self.get_url(url, transform_html).await
    }
}
