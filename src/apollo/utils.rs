use std::thread::sleep;

use chrono::{DateTime, Local};
use reqwest::blocking::Response;
use serde_json::Value;

pub fn to_resp_json(resp: Response) -> Result<Value, String> {
    let status = resp.status();
    let status_code = status.as_u16();
    let json = resp.json::<Value>().unwrap();
    let success = status.is_success() && !json.get("error").is_some();

    if success {
        Ok(json)
    } else {
        Err(format!(
            "[{}][{}] {}",
            status_code,
            if success { "Success" } else { "Failed" },
            json
        ))
    }
}

pub fn sleep_until(target: &DateTime<Local>) {
    let now = Local::now();
    let to_target_duration = target.signed_duration_since(now);

    match to_target_duration.to_std() {
        Ok(d) => {
            println!("now={}, sleeps {}s till {}", now, d.as_secs_f64(), target);
            sleep(d)
        }
        Err(_) => println!("now={}, target time {} already passed", now, target),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    #[ignore = "manual run only"]
    fn test_sleep_until() {
        let now = Local::now();
        sleep_until(&now.checked_add_signed(Duration::seconds(1)).unwrap());
        let after = Local::now();
        assert!(after.signed_duration_since(now).num_seconds() >= 1)
    }
}
