use std::process::Command;
//use dateparser::parse;
use chrono::{DateTime, TimeZone, Local, Utc};
use regex::Regex;


pub fn get_podman_image_refresh_time(img: &str ) -> Result<DateTime<Local>, String> {
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
            let dt = Local.with_ymd_and_hms(1900, 1, 1, 0, 0, 0).unwrap();
            return Ok(dt);
        }
        let stderr = String::from_utf8(output.stderr).map_err(|e| format!("Failed to parse podman output: {}", e))?;
        Err(format!("podman failed: {}", stderr))
    }
}

fn convert_str_to_date(date_str: &str) -> Result<DateTime<Local>, String> {
    let re = Regex::new(r"(?P<datetime>.+)(?P<tz_offset>[+-]\d{4}) (?P<tz_name>\w+)$").unwrap();
    let cleaned_date_str = re.replace(date_str, "$datetime$tz_offset");

    // Now try to parse the cleaned string
    match cleaned_date_str.parse::<DateTime<Utc>>() {
        Ok(parsed_date) => {
            //println!("Parsed DateTime: '{}'", parsed_date);
                Ok(parsed_date.with_timezone(&Local))
        }
        Err(e) => {
            println!("Failed to parse '{}': {:?}", date_str, e);
            todo!()
        }
    }

    //let date = DateTime::parse_from_rfc3339(date_str).map_err(|e| format!("Failed to parse date: {}", e))?;
    //Ok(date.with_timezone(&chrono::Utc))
}
