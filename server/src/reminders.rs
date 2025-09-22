use crate::{config::Config, notification, parse::parse_org_file};
use anyhow::Result;
use chrono::Local;
use org_parser::{Org, Reminder};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task, time};
use tracing::{debug, error, warn};
use walkdir::WalkDir;

async fn scan_reminders(path: &str, tx: mpsc::Sender<Org>) -> Result<()> {
    let now = Instant::now();
    let mut n = 0;
    let mut errors = 0;

    debug!("Starting scan of path: {}", path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path().to_owned();
        if let Some(ext) = path.extension()
            && ext == "org"
        {
            match parse_org_file(&path).await {
                Ok(org) => {
                    if let Err(err) = tx.send(org).await {
                        error!(
                            "Failed to send parsed org data for file {}: {:?}",
                            path.display(),
                            err
                        );
                        // チャンネルが閉じられている場合は処理を中断
                        break;
                    } else {
                        n += 1;
                        //debug!("Successfully processed file: {}", path.display());
                    }
                }
                Err(err) => {
                    error!("Failed to parse org file {}: {:?}", path.display(), err);
                    errors += 1;
                }
            }
        }
    }

    if errors > 0 {
        warn!(
            "Scan completed with {} errors out of {} total org files",
            errors,
            n + errors
        );
    } else {
        debug!(
            "Scan completed successfully: {} org files processed in {:?}",
            n,
            now.elapsed()
        );
    }

    Ok(())
}

pub fn scan(config: &Config, tx: &mpsc::Sender<Org>) {
    let mut handles = Vec::new();

    for p in &config.org_path {
        let p = p.clone();
        let tx = tx.clone();
        let handle = task::spawn(async move {
            if let Err(err) = scan_reminders(&p, tx).await {
                error!("Scan task failed for path {}: {:?}", p, err);
            }
        });
        handles.push(handle);
    }

    debug!("Spawned {} scan tasks for org paths", handles.len());

    // バックグラウンドでタスクの完了を待つ
    task::spawn(async move {
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(()) => debug!("Scan task {} completed successfully", i + 1),
                Err(err) => error!("Scan task {} join error: {:?}", i + 1, err),
            }
        }
        debug!("All scan tasks completed");
    });
}

/// 期限切れリマインダーをフィルタリングする関数（テスト用）
#[cfg(test)]
fn filter_expired_reminders(reminders: &[Reminder]) -> Vec<Reminder> {
    reminders
        .iter()
        .filter(|r| !r.is_expired())
        .cloned()
        .collect()
}

pub fn start_check(mut rx: mpsc::Receiver<Org>) {
    let _forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        let mut reminders: HashSet<Reminder> = HashSet::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // debug!("start check");
                    let now = Local::now().naive_local();
                    let mut temp = vec![];
                    let mut expired_reminders = vec![];

                    for val in &reminders {
                        if now > val.datetime {
                            // notify
                            let _ = notification::notify("Emacs Org Remainder", &val.title);
                            debug!("notify : {:?}", val);
                            temp.push(val.clone());  // remove entry
                        } else if val.is_expired() {
                            // 期限切れリマインダーを削除対象に追加
                            expired_reminders.push(val.clone());
                        }
                    }

                    // 通知済みリマインダーを削除
                    for val in temp {
                        reminders.remove(&val);
                    }

                    // 期限切れリマインダーを削除
                    for expired in expired_reminders {
                        if reminders.remove(&expired) {
                            debug!("Removed expired reminder: {:?}", expired);
                            match &expired.scheduling {
                                org_parser::Scheduling::Scheduled(_, _, datetime) => {
                                    warn!("Removed expired SCHEDULED reminder '{}' (scheduled for: {})",
                                          expired.title, datetime);
                                }
                                org_parser::Scheduling::Deadline(_, _, datetime) => {
                                    warn!("Removed expired DEADLINE reminder '{}' (deadline was: {})",
                                          expired.title, datetime);
                                }
                            }
                        }
                    }

                }
                data = rx.recv() => {
                    match data {
                        Some(org) => {
                            let res = org.get_reminders();
                            if !res.is_empty() {
                                let now = Local::now().naive_local();
                                for r in res {
                                    // 期限切れでない、かつ通知時刻が未来のリマインダーのみ追加
                                    if now < r.datetime && !r.is_expired() {
                                        let dr = r.clone();
                                        if reminders.insert(r) {
                                            debug!("append reminder: {:?}", &dr);
                                        }
                                    } else if r.is_expired() {
                                        // 期限切れリマインダーはログに記録して追加しない
                                        match &r.scheduling {
                                            org_parser::Scheduling::Scheduled(_, _, datetime) => {
                                                debug!("Skipped expired SCHEDULED reminder '{}' (scheduled for: {})",
                                                      r.title, datetime);
                                            }
                                            org_parser::Scheduling::Deadline(_, _, datetime) => {
                                                debug!("Skipped expired DEADLINE reminder '{}' (deadline was: {})",
                                                      r.title, datetime);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            debug!("Reminder channel closed, stopping reminder checker");
                            break;
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use org_parser::{Pos, Reminder, Scheduling};
    use std::time::Duration;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_filter_expired_reminders() {
        init();

        // 過去のSCHEDULED（期限切れ）
        let expired_scheduled = Reminder {
            title: "期限切れスケジュール".to_string(),
            datetime: Local::now().naive_local() - Duration::from_secs(3600), // 1時間前
            scheduling: Scheduling::Scheduled(
                Pos::new(0, 0),
                "期限切れスケジュール".to_string(),
                "2020-01-01 Wed 10:00".to_string(),
            ),
        };

        // 過去のDEADLINE（期限切れ）
        let expired_deadline = Reminder {
            title: "期限切れ締切".to_string(),
            datetime: Local::now().naive_local() - Duration::from_secs(3600), // 1時間前
            scheduling: Scheduling::Deadline(
                Pos::new(0, 0),
                "期限切れ締切".to_string(),
                "2020-01-01 Wed 23:59".to_string(),
            ),
        };

        // 未来のSCHEDULED（有効）
        let valid_scheduled = Reminder {
            title: "有効なスケジュール".to_string(),
            datetime: Local::now().naive_local() + Duration::from_secs(3600), // 1時間後
            scheduling: Scheduling::Scheduled(
                Pos::new(0, 0),
                "有効なスケジュール".to_string(),
                "2030-01-01 Wed 10:00".to_string(),
            ),
        };

        // 未来のDEADLINE（有効）
        let valid_deadline = Reminder {
            title: "有効な締切".to_string(),
            datetime: Local::now().naive_local() + Duration::from_secs(3600), // 1時間後
            scheduling: Scheduling::Deadline(
                Pos::new(0, 0),
                "有効な締切".to_string(),
                "2030-01-01 Wed 23:59".to_string(),
            ),
        };

        let all_reminders = vec![
            expired_scheduled,
            expired_deadline,
            valid_scheduled.clone(),
            valid_deadline.clone(),
        ];

        let filtered = filter_expired_reminders(&all_reminders);

        // 期限切れでないリマインダーのみが残ることを確認
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&valid_scheduled));
        assert!(filtered.contains(&valid_deadline));
    }

    #[test]
    fn test_filter_expired_reminders_empty() {
        init();

        let empty_reminders: Vec<Reminder> = vec![];
        let filtered = filter_expired_reminders(&empty_reminders);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_expired_reminders_all_expired() {
        init();

        // 全て期限切れのリマインダー
        let expired_reminders = vec![
            Reminder {
                title: "期限切れ1".to_string(),
                datetime: Local::now().naive_local() - Duration::from_secs(3600),
                scheduling: Scheduling::Scheduled(
                    Pos::new(0, 0),
                    "期限切れ1".to_string(),
                    "2020-01-01 Wed 10:00".to_string(),
                ),
            },
            Reminder {
                title: "期限切れ2".to_string(),
                datetime: Local::now().naive_local() - Duration::from_secs(7200),
                scheduling: Scheduling::Deadline(
                    Pos::new(0, 0),
                    "期限切れ2".to_string(),
                    "2020-01-01 Wed 23:59".to_string(),
                ),
            },
        ];

        let filtered = filter_expired_reminders(&expired_reminders);

        // 全て期限切れなので、結果は空になる
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_expired_reminders_all_valid() {
        init();

        // 全て有効なリマインダー
        let valid_reminders = vec![
            Reminder {
                title: "有効1".to_string(),
                datetime: Local::now().naive_local() + Duration::from_secs(3600),
                scheduling: Scheduling::Scheduled(
                    Pos::new(0, 0),
                    "有効1".to_string(),
                    "2030-01-01 Wed 10:00".to_string(),
                ),
            },
            Reminder {
                title: "有効2".to_string(),
                datetime: Local::now().naive_local() + Duration::from_secs(7200),
                scheduling: Scheduling::Deadline(
                    Pos::new(0, 0),
                    "有効2".to_string(),
                    "2030-01-01 Wed 23:59".to_string(),
                ),
            },
        ];

        let filtered = filter_expired_reminders(&valid_reminders);

        // 全て有効なので、全てが残る
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered, valid_reminders);
    }
}
