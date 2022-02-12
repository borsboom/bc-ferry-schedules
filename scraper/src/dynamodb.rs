use crate::imports::*;
use crate::sailings::*;
use crate::types::*;
use crate::utils::*;
use ::aws_sdk_dynamodb::model::AttributeValue;

const DEFAULT_DYNAMODB_AWS_REGION: &str = "us-west-2";

fn weekday_dynamodb_code(wd: Weekday) -> &'static str {
    match wd {
        Weekday::Sun => "sun",
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
    }
}

impl RouteGroup {
    fn dynamodb_code(&self) -> &'static str {
        match self {
            RouteGroup::SaltSpringAndOuterGulfIslands => "ssogi",
        }
    }
}

impl Terminal {
    fn dynamodb_code(&self) -> &'static str {
        match self {
            Terminal::GalianoIslandSturdiesBay => "gisb",
            Terminal::MayneIslandVillageBay => "mivb",
            Terminal::VancouverTsawwassen => "vats",
            Terminal::PenderIslandOtterBay => "piob",
            Terminal::SaltSpringIslandLongHarbour => "sslh",
            Terminal::SaturnaIslandLyallHarbour => "silh",
            Terminal::VictoriaSwartzBay => "visb",
        }
    }
}

impl StopType {
    fn dynamodb_code(&self) -> &'static str {
        match self {
            StopType::Stop => "stop",
            StopType::Transfer => "transfer",
        }
    }
}

impl ScheduleWeekday {
    fn to_dynamodb(&self) -> HashMap<String, AttributeValue> {
        HashMap::from_iter([(
            "onlyDates".to_string(),
            AttributeValue::L(self.only_dates.iter().map(|d| AttributeValue::S(format_iso_date(*d))).collect()),
        )])
    }
}

impl Stop {
    fn to_dynamodb(self) -> HashMap<String, AttributeValue> {
        HashMap::from_iter([
            ("type".to_string(), AttributeValue::S(self.type_.dynamodb_code().to_string())),
            ("terminal".to_string(), AttributeValue::S(self.terminal.dynamodb_code().to_string())),
        ])
    }
}

impl Sailing {
    fn to_dynamodb(&self) -> HashMap<String, AttributeValue> {
        HashMap::from_iter([
            ("departureTime".to_string(), AttributeValue::S(format_hours_minutes(self.depart_time))),
            ("arrivalTime".to_string(), AttributeValue::S(format_hours_minutes(self.arrive_time))),
            (
                "stops".to_string(),
                AttributeValue::L(self.stops.iter().map(|s| AttributeValue::M(s.to_dynamodb())).collect()),
            ),
        ])
    }
}

impl ScheduleItem {
    fn to_dynamodb(&self) -> HashMap<String, AttributeValue> {
        HashMap::from_iter([
            ("sailing".to_string(), AttributeValue::M(self.sailing.to_dynamodb())),
            (
                "exceptDates".to_string(),
                AttributeValue::L(self.except_dates.iter().map(|d| AttributeValue::S(format_iso_date(*d))).collect()),
            ),
            (
                "weekdays".to_string(),
                AttributeValue::M(
                    self.weekdays
                        .iter()
                        .map(|(w, sw)| (weekday_dynamodb_code(*w).to_string(), AttributeValue::M(sw.to_dynamodb())))
                        .collect(),
                ),
            ),
        ])
    }
}

impl Schedule {
    fn to_dynamodb(&self) -> HashMap<String, AttributeValue> {
        HashMap::from_iter([
            (
                "departureTerminal,arrivalTerminal".to_string(),
                AttributeValue::S(format!(
                    "{},{}",
                    self.terminal_pair.from.dynamodb_code(),
                    self.terminal_pair.to.dynamodb_code()
                )),
            ),
            ("effectiveFromDate".to_string(), AttributeValue::S(format_iso_date(self.effective_date_range.from))),
            (
                "effectiveToDate,departureTerminal,arrivalTerminal".to_string(),
                AttributeValue::S(format!(
                    "{},{},{}",
                    format_iso_date(self.effective_date_range.to),
                    self.terminal_pair.from.dynamodb_code(),
                    self.terminal_pair.to.dynamodb_code()
                )),
            ),
            (
                "effectiveFromDate,departureTerminal,arrivalTerminal".to_string(),
                AttributeValue::S(format!(
                    "{},{},{}",
                    format_iso_date(self.effective_date_range.from),
                    self.terminal_pair.from.dynamodb_code(),
                    self.terminal_pair.to.dynamodb_code()
                )),
            ),
            ("effectiveToDate".to_string(), AttributeValue::S(format_iso_date(self.effective_date_range.to))),
            ("arrivalTerminal".to_string(), AttributeValue::S(self.terminal_pair.to.dynamodb_code().to_string())),
            ("departureTerminal".to_string(), AttributeValue::S(self.terminal_pair.from.dynamodb_code().to_string())),
            ("sourceURL".to_string(), AttributeValue::S(self.source_url.to_string())),
            ("routeGroup".to_string(), AttributeValue::S(self.route_group.dynamodb_code().to_string())),
            ("reservable".to_string(), AttributeValue::Bool(self.reservable)),
            (
                "items".to_string(),
                AttributeValue::L(self.items.iter().map(|si| AttributeValue::M(si.to_dynamodb())).collect()),
            ),
        ])
    }

    async fn put_dynamodb(&self, dynamodb_client: &aws_sdk_dynamodb::Client) -> Result<()> {
        let request = dynamodb_client.put_item().table_name("ferrysched-schedules").set_item(Some(self.to_dynamodb()));
        request.send().await?;
        Ok(())
    }
}

impl Sailings {
    fn to_dynamodb(&self) -> HashMap<String, AttributeValue> {
        HashMap::from_iter([
            (
                "departureTerminal,arrivalTerminal".to_string(),
                AttributeValue::S(format!(
                    "{},{}",
                    self.terminal_pair.from.dynamodb_code(),
                    self.terminal_pair.to.dynamodb_code()
                )),
            ),
            ("departureTerminal".to_string(), AttributeValue::S(self.terminal_pair.from.dynamodb_code().to_string())),
            ("arrivalTerminal".to_string(), AttributeValue::S(self.terminal_pair.to.dynamodb_code().to_string())),
            ("date".to_string(), AttributeValue::S(format_iso_date(self.date))),
            (
                "date,routeGroup".to_string(),
                AttributeValue::S(format!(
                    "{},{}",
                    format_iso_date(self.date),
                    RouteGroup::SaltSpringAndOuterGulfIslands.dynamodb_code()
                )),
            ),
            (
                "sailings".to_string(),
                AttributeValue::L(self.sailings.iter().map(|s| AttributeValue::M(s.to_dynamodb())).collect()),
            ),
        ])
    }

    async fn put_dynamodb(&self, dynamodb_client: &aws_sdk_dynamodb::Client) -> Result<()> {
        let request = dynamodb_client.put_item().table_name("ferrysched-sailings").set_item(Some(self.to_dynamodb()));
        request.send().await?;
        Ok(())
    }
}
pub async fn put_dynamodb(schedules: &[Schedule]) -> Result<()> {
    let aws_region_provider =
        aws_config::meta::region::RegionProviderChain::default_provider().or_else(DEFAULT_DYNAMODB_AWS_REGION);
    let aws_config = aws_config::from_env().region(aws_region_provider).load().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
    for schedule in schedules {
        info!(
            "Putting to DynamoDB: {} to {}, {} - {}",
            schedule.terminal_pair.from.dynamodb_code(),
            schedule.terminal_pair.to.dynamodb_code(),
            format_iso_date(schedule.effective_date_range.from),
            format_iso_date(schedule.effective_date_range.to)
        );
        schedule.put_dynamodb(&dynamodb_client).await?;
        let sailings = Sailings::from_schedule(schedule)?;
        for date_sailings in sailings {
            date_sailings.put_dynamodb(&dynamodb_client).await?;
        }
    }
    Ok(())
}
