use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, Timelike, Weekday};

pub fn now_local() -> DateTime<Local> {
    Local::now()
}

pub fn format_time(t: &NaiveTime) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        t.hour(),
        t.minute(),
        t.second()
    )
}

pub fn format_date(d: &NaiveDate) -> String {
    format!("{:02}/{:02}/{:04}", d.day(), d.month(), d.year())
}

pub fn weekday_name(d: &NaiveDate) -> &'static str {
    match d.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn format_time_zero_pads_single_digits() {
        assert_eq!(
            format_time(&NaiveTime::from_hms_opt(0, 0, 7).unwrap()),
            "00:00:07"
        );
        assert_eq!(
            format_time(&NaiveTime::from_hms_opt(9, 5, 3).unwrap()),
            "09:05:03"
        );
    }

    #[test]
    fn format_time_handles_end_of_day() {
        assert_eq!(
            format_time(&NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
            "23:59:59"
        );
    }

    #[test]
    fn format_date_is_day_month_year() {
        assert_eq!(
            format_date(&NaiveDate::from_ymd_opt(1999, 12, 31).unwrap()),
            "31/12/1999"
        );
    }

    #[test]
    fn weekday_name_full_english_names() {
        assert_eq!(
            weekday_name(&NaiveDate::from_ymd_opt(1999, 12, 31).unwrap()),
            "Friday"
        );
        assert_eq!(
            weekday_name(&NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()),
            "Monday"
        );
    }
}
