use crate::imports::*;
use crate::types::*;

const ISO8601_DATE_FORMAT: &TimeFormat = format_description!("[year]-[month]-[day]");

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"), feature = "wasmbind")))]
pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi"), feature = "wasmbind"))]
pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(1000000i128 * (stdweb::web::Date::now() as i128)).unwrap()
}

pub fn now_vancouver() -> OffsetDateTime {
    now_utc().to_timezone(timezones::db::america::VANCOUVER)
}

pub fn today_vancouver() -> Date {
    now_vancouver().date()
}

pub fn format_iso8601_date(date: Date) -> String {
    date.format(ISO8601_DATE_FORMAT).unwrap()
}

pub fn parse_iso8601_date(input: &str) -> Result<Date> {
    Date::parse(input, ISO8601_DATE_FORMAT).context("Invalid date format (expect YYYY-MM-DD)")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_iso8601_date() -> Result<()> {
        assert_eq!(format_iso8601_date(date!(2021 - 03 - 31)), "2021-03-31");
        Ok(())
    }

    #[test]
    fn test_parse_iso8601_date() -> Result<()> {
        assert_eq!(parse_iso8601_date("2021-03-31")?, date!(2021 - 03 - 31));
        Ok(())
    }
}
