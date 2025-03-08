use chrono::{DateTime, Local, TimeZone, Utc};
use regex::Regex;
use std::process::Command;

pub fn get_podman_image_upstream_create_time(img: &str) -> Result<DateTime<Local>, String> {
    let mut cmd = Command::new("podman");
    cmd.arg("image");
    cmd.arg("inspect");
    cmd.arg("--format");
    cmd.arg("{{.Created}}");
    cmd.arg(img);
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute podman: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse podman output: {}", e))?;
        let x = convert_str_to_date(stdout.trim());
        Ok(x?)
    } else {
        // if error = image not known, then just return 1/1/1900
        if std::str::from_utf8(&output.stderr)
            .unwrap()
            .contains("image not known")
        {
            let dt = Local.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap();
            return Ok(dt);
        }
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| format!("Failed to parse podman output: {}", e))?;
        Err(format!("podman failed: {}", stderr))
    }
}

pub fn get_podman_ondisk_modify_time(img: &str) -> Result<DateTime<Local>, String> {
    let mut cmd = Command::new("podman");
    cmd.arg("image");
    cmd.arg("inspect");
    cmd.arg("--format");
    cmd.arg("{{.Id}}");
    cmd.arg(img);
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute podman: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse podman output: {}", e))?;
        // let x = convert_str_to_date(stdout.trim());
        // Ok(x?)
        let id = stdout.trim().to_string();

        let homedir = std::env::var("HOME").unwrap();
        let path = format!(
            "{}/.local/share/containers/storage/overlay-images/{}/manifest",
            homedir, id
        );
        let mut cmd2 = Command::new("stat");
        cmd2.arg("-c");
        cmd2.arg("%y");
        cmd2.arg(path);
        let output2 = cmd2
            .output()
            .map_err(|e| format!("Failed to execute stat: {}", e))?;

        if output2.status.success() {
            let stdout2 = String::from_utf8(output2.stdout)
                .map_err(|e| format!("Failed to parse stat output: {}", e))?;
            let x = convert_str_to_date(stdout2.trim());
            Ok(x?)
        } else {
            let stderr = String::from_utf8(output2.stderr)
                .map_err(|e| format!("Failed to parse stat output: {}", e))?;
            Err(format!("stat failed: {}", stderr))
        }
    } else {
        // if error = image not known, then just return 1/1/1900
        if std::str::from_utf8(&output.stderr)
            .unwrap()
            .contains("image not known")
        {
            let dt = Local.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap();
            Ok(dt)
        } else {
            let stderr = String::from_utf8(output.stderr)
                .map_err(|e| format!("Failed to parse podman output: {}", e))?;
            Err(format!("podman failed: {}", stderr))
        }
    }
}

fn convert_str_to_date(date_str: &str) -> Result<DateTime<Local>, String> {
    // Had to fix this up based on this create date gunk
    // Is that set upstream? Whether its upstream or my computer, let's just fix it in parse 😑
    // $ podman image inspect --format {{.Created}} docker.io/linuxserver/wireguard:arm64v8-latest
    // 2024-10-03 12:28:30.701255218 +0100 +0100

    let re = Regex::new(r"(?P<datetime>[0-9:\-\s\.]+)(?P<tz_offset>[+-]\d{4})")
        .map_err(|e| format!("Failed to compile regex: {}", e))?;
    let captures = re.captures(date_str);
    // dbg!(&captures);
    let tz_offset = match captures.as_ref() {
        Some(caps) => caps["tz_offset"].to_string(),
        None => {
            return Err(format!(
                "Failed to parse timezone offset from '{}'",
                date_str
            ));
        }
    };
    // dbg!(&tz_offset);
    let mut cleaned_date_str;
    match captures {
        Some(caps) => {
            if !caps["datetime"].is_empty() {
                cleaned_date_str = caps["datetime"].replace("T", " ");
            } else {
                return Err(format!(
                    "Failed to parse timezone offset from '{}'",
                    date_str
                ));
            }
            if !tz_offset.is_empty() {
                cleaned_date_str = format!("{}{}", cleaned_date_str, tz_offset);
            } else {
                cleaned_date_str = format!("{}{}", cleaned_date_str, "+0000");
            }
        }
        None => {
            return Err(format!("Failed to parse date from '{}'", date_str));
        }
    }

    // Now try to parse the cleaned string
    match cleaned_date_str.parse::<DateTime<Utc>>() {
        Ok(parsed_date) => {
            //println!("Parsed DateTime: '{}'", parsed_date);
            Ok(parsed_date.with_timezone(&Local))
        }
        Err(e) => Err(format!("Failed to parse date '{}': {}", date_str, e)),
    }
}
