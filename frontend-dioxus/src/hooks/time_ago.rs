use chrono::{DateTime, Utc};
use dioxus::prelude::*;

pub fn use_time_ago(datetime: DateTime<Utc>) -> Signal<String> {
    let mut time_str = use_signal(|| format_time_ago(datetime));

    use_effect(move || {
        spawn(async move {
            loop {
                gloo_timers::future::sleep(std::time::Duration::from_secs(30)).await;
                time_str.set(format_time_ago(datetime));
            }
        });
    });

    time_str
}

fn format_time_ago(datetime: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(datetime);

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{} minute{} ago", minutes, if minutes == 1 { "" } else { "s" })
    } else if hours < 24 {
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if days < 30 {
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if days < 365 {
        let months = days / 30;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = days / 365;
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}
