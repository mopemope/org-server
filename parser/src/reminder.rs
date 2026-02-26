use crate::parser::{Scheduling, Section};
use chrono::prelude::*;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Clone, Debug, Eq)]
pub struct Reminder {
    pub title: String,
    pub datetime: NaiveDateTime,
    pub scheduling: Scheduling,
    pub filename: Option<String>,
}

impl PartialEq for Reminder {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.datetime == other.datetime
            && self.filename == other.filename
    }
}

impl Hash for Reminder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.datetime.hash(state);
        self.filename.hash(state);
    }
}

impl Reminder {
    /// リマインダーが期限切れかどうかを判定する
    /// SCHEDULEDまたはDEADLINEの期限が現在時刻を過ぎている場合にtrueを返す
    pub fn is_expired(&self) -> bool {
        let now = Local::now().naive_local();

        match &self.scheduling {
            Scheduling::Scheduled(_, _, datetime_str) => {
                // SCHEDULEDの期限チェック
                if let Some(scheduled_time) = parse_scheduling_datetime(datetime_str) {
                    return scheduled_time < now;
                }
            }
            Scheduling::Deadline(_, _, datetime_str) => {
                // DEADLINEの期限チェック
                if let Some(deadline_time) = parse_scheduling_datetime(datetime_str) {
                    return deadline_time < now;
                }
            }
        }

        false
    }
}

/// スケジューリング文字列をNaiveDateTimeにパースする
pub fn parse_scheduling_datetime(datetime_str: &str) -> Option<NaiveDateTime> {
    let clean_datetime =
        datetime_str.trim_matches(|c| c == '<' || c == '>' || c == '[' || c == ']');
    let parts: Vec<&str> = clean_datetime.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    let date_part = parts[0];
    let time_part = parts
        .iter()
        .find(|&p| p.contains(':'))
        .map(|p| if p.len() >= 5 { &p[0..5] } else { *p })
        .unwrap_or("09:00");

    let std_datetime = format!("{} {}", date_part, time_part);

    // まず時刻付きの形式を試す
    if let Ok(dt) = NaiveDateTime::parse_from_str(&std_datetime, "%Y-%m-%d %H:%M") {
        return Some(dt);
    }

    // 時刻なしの場合はデフォルト時刻（09:00）を追加
    let datetime_with_time = format!("{std_datetime} 09:00");
    if let Ok(dt) = NaiveDateTime::parse_from_str(&datetime_with_time, "%Y-%m-%d %H:%M") {
        return Some(dt);
    }

    None
}

/// デフォルトのリマインダー間隔（分）
pub const DEFAULT_REMINDER_INTERVALS: &[u32] = &[30, 10, 1];

/// カスタマイズ可能なリマインダー設定
#[derive(Clone, Debug)]
pub struct ReminderConfig {
    pub intervals_minutes: Vec<u32>,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            intervals_minutes: DEFAULT_REMINDER_INTERVALS.to_vec(),
        }
    }
}

pub fn get_reminders(sec: &Section) -> Vec<Reminder> {
    get_reminders_with_config(sec, &ReminderConfig::default())
}

pub fn get_reminders_with_config(sec: &Section, config: &ReminderConfig) -> Vec<Reminder> {
    let mut res = vec![];
    for sch in &sec.scheduling {
        if let Some(mut reminders) = convert_reminder_with_config(sch, config) {
            res.append(&mut reminders);
        }
    }
    for sec in &sec.sections {
        let mut reminders = get_reminders_with_config(sec, config);
        if !reminders.is_empty() {
            res.append(&mut reminders);
        }
    }
    res
}

fn create_reminder_with_config(
    dt: NaiveDateTime,
    sch: &Scheduling,
    config: &ReminderConfig,
) -> Vec<Reminder> {
    let mut vec = vec![];

    for &interval_minutes in &config.intervals_minutes {
        let title = match sch {
            Scheduling::Deadline(_, title, _) => {
                if interval_minutes >= 60 {
                    let hours = interval_minutes / 60;
                    let remaining_minutes = interval_minutes % 60;
                    if remaining_minutes == 0 {
                        format!("このイベント終了まであと{hours}時間: {title}")
                    } else {
                        format!("このイベント終了まであと{hours}時間{remaining_minutes}分: {title}")
                    }
                } else {
                    format!("このイベント終了まであと{interval_minutes}分: {title}")
                }
            }
            Scheduling::Scheduled(_, title, _) => {
                if interval_minutes >= 60 {
                    let hours = interval_minutes / 60;
                    let remaining_minutes = interval_minutes % 60;
                    if remaining_minutes == 0 {
                        format!("このイベント開始まであと{hours}時間: {title}")
                    } else {
                        format!("このイベント開始まであと{hours}時間{remaining_minutes}分: {title}")
                    }
                } else {
                    format!("このイベント開始まであと{interval_minutes}分: {title}")
                }
            }
        };

        let reminder_datetime = dt - Duration::from_secs(60 * u64::from(interval_minutes));
        let rem = Reminder {
            title,
            datetime: reminder_datetime,
            scheduling: sch.clone(),
            filename: None,
        };
        vec.push(rem);
    }
    vec
}

pub fn convert_reminder_with_config(
    sch: &Scheduling,
    config: &ReminderConfig,
) -> Option<Vec<Reminder>> {
    let now = Local::now().naive_local();
    match sch {
        Scheduling::Scheduled(_, _title, datetime) => {
            parse_datetime_and_create_reminder(datetime, sch, config, now)
        }
        Scheduling::Deadline(_, _title, datetime) => {
            parse_datetime_and_create_reminder(datetime, sch, config, now)
        }
    }
}

fn parse_datetime_and_create_reminder(
    datetime: &str,
    sch: &Scheduling,
    config: &ReminderConfig,
    now: NaiveDateTime,
) -> Option<Vec<Reminder>> {
    let clean_datetime = datetime.trim_matches(|c| c == '<' || c == '>' || c == '[' || c == ']');
    let parts: Vec<&str> = clean_datetime.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    let date_part = parts[0];
    let time_part = parts
        .iter()
        .find(|&p| p.contains(':'))
        .map(|p| if p.len() >= 5 { &p[0..5] } else { *p })
        .unwrap_or("09:00");

    let std_datetime = format!("{} {}", date_part, time_part);

    // まず時刻付きの形式を試す
    if let Ok(dt) = NaiveDateTime::parse_from_str(&std_datetime, "%Y-%m-%d %H:%M")
        && dt > now
    {
        return Some(create_reminder_with_config(dt, sch, config));
    }

    // 時刻なしの場合はデフォルト時刻（09:00）を追加
    let datetime_with_time = format!("{std_datetime} 09:00");
    if let Ok(dt) = NaiveDateTime::parse_from_str(&datetime_with_time, "%Y-%m-%d %H:%M")
        && dt > now
    {
        return Some(create_reminder_with_config(dt, sch, config));
    }

    None
}

// 後方互換性のための関数（テスト用）
#[cfg(test)]
fn convert_reminder(sch: &Scheduling) -> Option<Vec<Reminder>> {
    convert_reminder_with_config(sch, &ReminderConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Pos;
    use tracing::debug;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_convert_reminder() {
        // SCHEDULED: <2035-03-04 Tue 10:00>
        init();
        let pos = Pos::new(0, 0);
        let rem = convert_reminder(&Scheduling::Scheduled(
            pos,
            "title".to_string(),
            "2035-03-04 Tue 13:00".to_string(),
        ));
        debug!("{:?}", rem);

        let pos = Pos::new(0, 0);
        let rem = convert_reminder(&Scheduling::Scheduled(
            pos,
            "title".to_string(),
            "2035-03-04 Tue".to_string(),
        ));
        debug!("{:?}", rem);
    }

    #[test]
    fn test_parse_scheduling_datetime_edge_cases() {
        init();
        let cases = vec![
            ("2035-03-04", "2035-03-04 09:00:00"),
            ("2035-03-04 Tue", "2035-03-04 09:00:00"),
            ("2035-03-04 Tue 13:00", "2035-03-04 13:00:00"),
            ("2035-03-04 13:00", "2035-03-04 13:00:00"),
            ("2035-03-04 Tue 13:00-14:00", "2035-03-04 13:00:00"),
            ("2035-03-04 Tue +1w", "2035-03-04 09:00:00"),
            ("2035-03-04 Tue 13:00 +1w", "2035-03-04 13:00:00"),
            ("2035-03-04 13:00-14:00 +1w", "2035-03-04 13:00:00"),
        ];

        for (input, expected) in cases {
            let dt = parse_scheduling_datetime(input)
                .unwrap_or_else(|| panic!("Failed to parse: {}", input));
            let expected_dt = NaiveDateTime::parse_from_str(expected, "%Y-%m-%d %H:%M:%S").unwrap();
            assert_eq!(dt, expected_dt, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_custom_reminder_config() {
        init();
        let config = ReminderConfig {
            intervals_minutes: vec![60, 30, 5], // 1時間前、30分前、5分前
        };

        let pos = Pos::new(0, 0);
        let rem = convert_reminder_with_config(
            &Scheduling::Scheduled(
                pos,
                "重要な会議".to_string(),
                "2035-12-25 Thu 14:00".to_string(),
            ),
            &config,
        );

        if let Some(reminders) = rem {
            assert_eq!(reminders.len(), 3);
            assert!(reminders[0].title.contains("1時間"));
            assert!(reminders[1].title.contains("30分"));
            assert!(reminders[2].title.contains("5分"));
        }
    }

    #[test]
    fn test_is_expired_scheduled() {
        init();

        // 過去のSCHEDULED（期限切れ）
        let past_scheduled = Reminder {
            title: "過去のイベント".to_string(),
            datetime: Local::now().naive_local() - Duration::from_secs(3600), // 1時間前
            scheduling: Scheduling::Scheduled(
                Pos::new(0, 0),
                "過去のイベント".to_string(),
                "2020-01-01 Wed 10:00".to_string(),
            ),
            filename: None,
        };
        assert!(past_scheduled.is_expired());

        // 未来のSCHEDULED（期限内）
        let future_scheduled = Reminder {
            title: "未来のイベント".to_string(),
            datetime: Local::now().naive_local() + Duration::from_secs(3600), // 1時間後
            scheduling: Scheduling::Scheduled(
                Pos::new(0, 0),
                "未来のイベント".to_string(),
                "2030-01-01 Wed 10:00".to_string(),
            ),
            filename: None,
        };
        assert!(!future_scheduled.is_expired());
    }

    #[test]
    fn test_is_expired_deadline() {
        init();

        // 過去のDEADLINE（期限切れ）
        let past_deadline = Reminder {
            title: "過去の締切".to_string(),
            datetime: Local::now().naive_local() - Duration::from_secs(3600), // 1時間前
            scheduling: Scheduling::Deadline(
                Pos::new(0, 0),
                "過去の締切".to_string(),
                "2020-01-01 Wed 23:59".to_string(),
            ),
            filename: None,
        };
        assert!(past_deadline.is_expired());

        // 未来のDEADLINE（期限内）
        let future_deadline = Reminder {
            title: "未来の締切".to_string(),
            datetime: Local::now().naive_local() + Duration::from_secs(3600), // 1時間後
            scheduling: Scheduling::Deadline(
                Pos::new(0, 0),
                "未来の締切".to_string(),
                "2030-01-01 Wed 23:59".to_string(),
            ),
            filename: None,
        };
        assert!(!future_deadline.is_expired());
    }

    #[test]
    fn test_is_expired_date_only() {
        init();

        // 日付のみ（時刻なし）の過去のSCHEDULED
        let past_date_only = Reminder {
            title: "過去の日付のみ".to_string(),
            datetime: Local::now().naive_local() - Duration::from_secs(86400), // 1日前
            scheduling: Scheduling::Scheduled(
                Pos::new(0, 0),
                "過去の日付のみ".to_string(),
                "2020-01-01 Wed".to_string(),
            ),
            filename: None,
        };
        assert!(past_date_only.is_expired());

        // 日付のみ（時刻なし）の未来のDEADLINE
        let future_date_only = Reminder {
            title: "未来の日付のみ".to_string(),
            datetime: Local::now().naive_local() + Duration::from_secs(86400), // 1日後
            scheduling: Scheduling::Deadline(
                Pos::new(0, 0),
                "未来の日付のみ".to_string(),
                "2030-01-01 Wed".to_string(),
            ),
            filename: None,
        };
        assert!(!future_date_only.is_expired());
    }
}
