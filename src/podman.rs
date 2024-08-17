use std::process::Command;
use chrono::{DateTime, TimeZone, Utc};



pub fn get_podman_image_refresh_time(img: &str ) -> Result<DateTime<Utc>, String> {
    let mut cmd = Command::new("podman");
    cmd.arg("image");
    cmd.arg("inspect");
    cmd.arg("--format");
    cmd.arg("{{.Created}}");
    cmd.arg(img);
    let output = cmd.output().map_err(|e| format!("Failed to execute podman: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(|e| format!("Failed to parse podman output: {}", e))?;
        let x = convert_str_to_date(stdout.trim());
        Ok(x?)
    } else {
        // if error = image not known, then just return 1/1/1900
        if std::str::from_utf8(&output.stderr).unwrap().contains("image not known") {
            let dt = Utc.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap();
            return Ok(dt);
        }
        let stderr = String::from_utf8(output.stderr).map_err(|e| format!("Failed to parse podman output: {}", e))?;
        Err(format!("podman failed: {}", stderr))
    }
}

fn convert_str_to_date(date_str: &str) -> Result<DateTime<Utc>, String> {
    let date = DateTime::parse_from_rfc3339(date_str).map_err(|e| format!("Failed to parse date: {}", e))?;
    Ok(date.with_timezone(&chrono::Utc))
}