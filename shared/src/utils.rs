use crate::imports::*;

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd(y, m, d)
}

pub fn now_pacific() -> DateTime<Tz> {
    Utc::now().with_timezone(&Pacific)
}

pub fn today_pacific() -> NaiveDate {
    now_pacific().date().naive_local()
}

pub fn into_group_map<T, I, K, V, F>(iter: I, f: F) -> HashMap<K, Vec<V>>
where
    I: IntoIterator<Item = T>,
    K: Eq + Hash,
    F: Fn(T) -> (K, V),
{
    iter.into_iter().fold(HashMap::new(), |mut map, item| {
        let (key, value) = f(item);
        map.entry(key).or_insert_with(Vec::new).push(value);
        map
    })
}
