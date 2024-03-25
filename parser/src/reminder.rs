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

pub fn get_reminders(sec: &Section) -> Vec<Reminder> {
    let mut res = vec![];
    for sch in &sec.scheduling {
        if let Some(mut reminders) = convert_reminder(sch) {
            res.append(&mut reminders);
        }
    }
    for sec in &sec.sections {
        let mut reminders = get_reminders(sec);
        if !reminders.is_empty() {
            res.append(&mut reminders);
        }
    }
    res
}

fn create_reminder(dt: NaiveDateTime, sch: &Scheduling) -> Vec<Reminder> {
    let mut vec = vec![];

    let (t30, t10, t1) = match sch {
        Scheduling::Deadline(_, title, _) => (
            format!("このイベント終了まであと30分: {}", title),
            format!("このイベント終了 まであと10分: {}", title),
            format!("このイベント終了まであと1分: {}", title),
        ),
        Scheduling::Scheduled(_, title, _) => (
            format!("このイベント開始まであと30分: {}", title),
            format!("このイベント開始まであと10分: {}", title),
            format!("このイベント開始まであと1分: {}", title),
        ),
    };
    let rem = Reminder {
        title: t30,
        datetime: dt - Duration::from_secs(60 * 30),
        scheduling: sch.clone(),
    };
    vec.push(rem);

    let rem = Reminder {
        title: t10,
        datetime: dt - Duration::from_secs(60 * 10),
        scheduling: sch.clone(),
    };
    vec.push(rem);

    let rem = Reminder {
        title: t1,
        datetime: dt - Duration::from_secs(60),
        scheduling: sch.clone(),
    };
    vec.push(rem);
    vec
}

// TODO refactor
fn convert_reminder(sch: &Scheduling) -> Option<Vec<Reminder>> {
    let now = Local::now().naive_local();
    match sch {
        Scheduling::Scheduled(_, _title, ref datetime) => {
            let dt = NaiveDateTime::parse_from_str(datetime, "%F %a %R");

            if let Ok(dt) = dt {
                if dt > now {
                    Some(create_reminder(dt, sch))
                } else {
                    None
                }
            } else {
                let datetime = format!("{} 09:00", &datetime);
                let dt = NaiveDateTime::parse_from_str(&datetime, "%F %a %R");
                if let Ok(dt) = dt {
                    if dt > now {
                        Some(create_reminder(dt, sch))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
        Scheduling::Deadline(_, _title, ref datetime) => {
            let dt = NaiveDateTime::parse_from_str(datetime, "%F %a %R");
            if let Ok(dt) = dt {
                if dt > now {
                    Some(create_reminder(dt, sch))
                } else {
                    None
                }
            } else {
                let datetime = format!("{} 09:00", &datetime);
                let dt = NaiveDateTime::parse_from_str(&datetime, "%F %a %R");
                if let Ok(dt) = dt {
                    if dt > now {
                        Some(create_reminder(dt, sch))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Pos;

    use super::*;
    use tracing::debug;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_convert_reminder() {
        // SCHEDULED: <2024-03-04 Mon 10:00>
        init();
        let pos = Pos::new(0, 0);
        let rem = convert_reminder(&Scheduling::Scheduled(
            pos,
            "title".to_string(),
            "2024-03-04 Mon 13:00".to_string(),
        ));
        debug!("{:?}", rem);

        let pos = Pos::new(0, 0);
        let rem = convert_reminder(&Scheduling::Scheduled(
            pos,
            "title".to_string(),
            "2024-03-04 Mon".to_string(),
        ));
        debug!("{:?}", rem);
    }
}
