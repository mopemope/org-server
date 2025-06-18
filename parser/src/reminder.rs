use crate::parser::{Scheduling, Section};
use chrono::prelude::*;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Clone, Debug, Eq)]
pub struct Reminder {
    pub title: String,
    pub datetime: NaiveDateTime,
    pub scheduling: Scheduling,
}

impl PartialEq for Reminder {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.datetime == other.datetime
    }
}

impl Hash for Reminder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.datetime.hash(state);
    }
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
                        format!("このイベント終了まであと{}時間: {}", hours, title)
                    } else {
                        format!(
                            "このイベント終了まであと{}時間{}分: {}",
                            hours, remaining_minutes, title
                        )
                    }
                } else {
                    format!("このイベント終了まであと{}分: {}", interval_minutes, title)
                }
            }
            Scheduling::Scheduled(_, title, _) => {
                if interval_minutes >= 60 {
                    let hours = interval_minutes / 60;
                    let remaining_minutes = interval_minutes % 60;
                    if remaining_minutes == 0 {
                        format!("このイベント開始まであと{}時間: {}", hours, title)
                    } else {
                        format!(
                            "このイベント開始まであと{}時間{}分: {}",
                            hours, remaining_minutes, title
                        )
                    }
                } else {
                    format!("このイベント開始まであと{}分: {}", interval_minutes, title)
                }
            }
        };

        let reminder_datetime = dt - Duration::from_secs(60 * interval_minutes as u64);
        let rem = Reminder {
            title,
            datetime: reminder_datetime,
            scheduling: sch.clone(),
        };
        vec.push(rem);
    }
    vec
}

fn convert_reminder_with_config(
    sch: &Scheduling,
    config: &ReminderConfig,
) -> Option<Vec<Reminder>> {
    let now = Local::now().naive_local();
    match sch {
        Scheduling::Scheduled(_, _title, ref datetime) => {
            parse_datetime_and_create_reminder(datetime, sch, config, now)
        }
        Scheduling::Deadline(_, _title, ref datetime) => {
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
    // まず時刻付きの形式を試す
    if let Ok(dt) = NaiveDateTime::parse_from_str(datetime, "%F %a %R") {
        if dt > now {
            return Some(create_reminder_with_config(dt, sch, config));
        }
    }

    // 時刻なしの場合はデフォルト時刻（09:00）を追加
    let datetime_with_time = format!("{} 09:00", datetime);
    if let Ok(dt) = NaiveDateTime::parse_from_str(&datetime_with_time, "%F %a %R") {
        if dt > now {
            return Some(create_reminder_with_config(dt, sch, config));
        }
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
        // SCHEDULED: <2025-03-04 Tue 10:00>
        init();
        let pos = Pos::new(0, 0);
        let rem = convert_reminder(&Scheduling::Scheduled(
            pos,
            "title".to_string(),
            "2025-03-04 Tue 13:00".to_string(),
        ));
        debug!("{:?}", rem);

        let pos = Pos::new(0, 0);
        let rem = convert_reminder(&Scheduling::Scheduled(
            pos,
            "title".to_string(),
            "2025-03-04 Tue".to_string(),
        ));
        debug!("{:?}", rem);
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
                "2025-12-25 Thu 14:00".to_string(),
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
}
