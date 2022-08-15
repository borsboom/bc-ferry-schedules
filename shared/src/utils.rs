use crate::imports::*;
use crate::types::*;

const ISO8601_DATE_FORMAT: &TimeFormat = format_description!("[year]-[month]-[day]");

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"), feature = "wasmbind")))]
pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi"), feature = "wasmbind"))]
pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(1000000i128 * (stdweb::web::Date::now() as i128))
        .expect("current timestamp to convert to offset date/time")
}

pub fn now_vancouver() -> OffsetDateTime {
    now_utc().to_timezone(timezones::db::america::VANCOUVER)
}

pub fn today_vancouver() -> Date {
    now_vancouver().date()
}

pub fn format_iso8601_date(date: Date) -> String {
    date.format(ISO8601_DATE_FORMAT).expect("iso8601 date to format")
}

pub fn parse_iso8601_date(input: &str) -> Result<Date> {
    Date::parse(input, ISO8601_DATE_FORMAT).context("Invalid date format (expect YYYY-MM-DD)")
}

pub fn into_group_map<T, In, Key, FKey, FNew, FIns, Out>(iter: In, f: FKey, n: FNew, p: FIns) -> HashMap<Key, Out>
where
    In: IntoIterator<Item = T>,
    Key: Eq + Hash,
    FKey: Fn(&T) -> Key,
    FNew: Fn() -> Out,
    FIns: Fn(&mut Out, T),
{
    iter.into_iter().fold(HashMap::new(), |mut map, item| {
        let key = f(&item);
        p(map.entry(key).or_insert_with(&n), item);
        map
    })
}

pub fn into_vec_group_map<T, In, Key, FKey>(iter: In, f: FKey) -> HashMap<Key, Vec<T>>
where
    In: IntoIterator<Item = T>,
    Key: Eq + Hash,
    FKey: Fn(&T) -> Key,
{
    into_group_map(iter, f, Vec::new, |v, i| v.push(i))
}

pub fn into_hashset_group_map<T, In, Key, FKey>(iter: In, f: FKey) -> HashMap<Key, HashSet<T>>
where
    T: Eq + Hash,
    In: IntoIterator<Item = T>,
    Key: Eq + Hash,
    FKey: Fn(&T) -> Key,
{
    into_group_map(iter, f, HashSet::new, |s, i| {
        s.insert(i);
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
