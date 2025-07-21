use chrono::{DateTime, Local};

#[derive(Debug, PartialEq)]
pub struct Image {
    pub name: Option<String>,
    pub container: Option<String>,
    pub skipall_by_this_name: bool,
}

/// Format a timestamp as a relative time (e.g., "5 minutes ago")
#[must_use]
pub fn format_time_ago(dt: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(dt);
    let days = duration.num_days();
    let hours = duration.num_hours();
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds();

    if days > 0 {
        format!("{days} days ago")
    } else if hours > 0 {
        format!("{hours} hours ago")
    } else if minutes > 0 {
        format!("{minutes} minutes ago")
    } else {
        format!("{seconds} seconds ago")
    }
}
