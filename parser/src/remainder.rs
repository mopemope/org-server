use crate::parser::{Scheduling, Section};
use chrono::prelude::*;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Clone, Debug, Eq)]
pub struct Remainder {
    pub title: String,
    pub datetime: NaiveDateTime,
    pub scheduling: Scheduling,
}

impl PartialEq for Remainder {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.datetime == other.datetime
    }
}

impl Hash for Remainder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.datetime.hash(state);
    }
}

pub fn get_remainders(sec: &Section) -> Vec<Remainder> {
    let mut res = vec![];
    for sch in &sec.scheduling {
        if let Some(mut reminders) = convert_remainder(&sec.title, sch) {
            res.append(&mut reminders);
        }
    }
    for sec in &sec.sections {
        let mut remainders = get_remainders(sec);
        if !remainders.is_empty() {
            res.append(&mut remainders);
        }
    }
    res
}

fn create_remainder(title: &str, dt: NaiveDateTime, sch: &Scheduling) -> Vec<Remainder> {
    let mut vec = vec![];

    let rem = Remainder {
        title: title.to_string(),
        datetime: dt - Duration::from_secs(60 * 30),
        scheduling: sch.clone(),
    };
    vec.push(rem);

    let rem = Remainder {
        title: title.to_string(),
        datetime: dt - Duration::from_secs(60 * 10),
        scheduling: sch.clone(),
    };
    vec.push(rem);

    let rem = Remainder {
        title: title.to_string(),
        datetime: dt - Duration::from_secs(60),
        scheduling: sch.clone(),
    };
    vec.push(rem);
    vec
}

// TODO refactor
fn convert_remainder(title: &str, sch: &Scheduling) -> Option<Vec<Remainder>> {
    let now = Local::now().naive_local();
    match sch {
        Scheduling::Scheduled(ref datetime) => {
            let dt = NaiveDateTime::parse_from_str(datetime, "%F %a %R");

            if let Ok(dt) = dt {
                if dt > now {
                    Some(create_remainder(title, dt, sch))
                } else {
                    None
                }
            } else {
                let datetime = format!("{} 09:00", &datetime);
                let dt = NaiveDateTime::parse_from_str(&datetime, "%F %a %R");
                if let Ok(dt) = dt {
                    if dt > now {
                        Some(create_remainder(title, dt, sch))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
        Scheduling::Deadline(ref datetime) => {
            let dt = NaiveDateTime::parse_from_str(datetime, "%F %a %R");
            if let Ok(dt) = dt {
                if dt > now {
                    Some(create_remainder(title, dt, sch))
                } else {
                    None
                }
            } else {
                let datetime = format!("{} 09:00", &datetime);
                let dt = NaiveDateTime::parse_from_str(&datetime, "%F %a %R");
                if let Ok(dt) = dt {
                    if dt > now {
                        Some(create_remainder(title, dt, sch))
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
    use super::*;
    use tracing::debug;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_convert_remainder() {
        // SCHEDULED: <2024-03-04 Mon 10:00>
        init();
        let rem = convert_remainder(
            "title",
            &Scheduling::Scheduled("2024-03-04 Mon 13:00".to_string()),
        );
        debug!("{:?}", rem);

        let rem = convert_remainder(
            "title",
            &Scheduling::Scheduled("2024-03-04 Mon".to_string()),
        );
        debug!("{:?}", rem);
    }
}
