use super::agent::PunchType;
use chrono::{DateTime, Duration, Local};
use rand::{thread_rng, Rng};
use serde_json::Value;
use std::fmt::Display;

pub struct WorkdaySchedule {
    date: String,
    work_on_time: Option<DateTime<Local>>,
    work_off_time: Option<DateTime<Local>>,
    memo: Option<String>,
}

impl Display for WorkdaySchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.date,
            self.description(),
            self.work_on_time
                .map_or("N/A".to_string(), |v| v.to_rfc3339()),
            self.work_off_time
                .map_or("N/A".to_string(), |v| v.to_rfc3339()),
        )
    }
}

fn parse_as_local_time(v: &Value) -> Option<DateTime<Local>> {
    return v.as_str().map_or(None, |v| {
        DateTime::parse_from_rfc3339(v)
            .map(|v| Some(v.with_timezone(&Local)))
            .unwrap()
    });
}

impl WorkdaySchedule {
    pub fn from_json(json: &Value) -> Self {
        // println!("{}", serde_json::to_string_pretty(&json).unwrap());

        let date = json["Date"]
            .as_str()
            .unwrap()
            .split_once('T')
            .map(|(first, _)| first.to_string())
            .unwrap();

        let work_on_time = parse_as_local_time(&json["ShiftSchedule"]["WorkOnTime"]);
        let work_off_time = parse_as_local_time(&json["ShiftSchedule"]["WorkOffTime"]);

        let memo = json["CalendarEvent"]["EventMemo"]
            .as_str()
            .map_or(None, |v| Some(v.to_string()));

        return WorkdaySchedule {
            date,
            work_on_time,
            work_off_time,
            memo,
        };
    }

    pub fn is_work_day(&self) -> bool {
        return self.work_on_time.is_some() || self.work_off_time.is_some();
    }

    pub fn description(&self) -> String {
        return format!(
            "{}{}",
            if self.is_work_day() {
                "工作日"
            } else {
                "休假日"
            },
            self.memo
                .as_ref()
                .map_or("".to_string(), |v| format!("({})", v)),
        );
    }

    pub fn get_date(&self) -> &str {
        self.date.as_str()
    }

    pub fn get_punch_time_with_jitter(
        &self,
        punch_type: PunchType,
        jitter: Option<u32>, // default = 60s
    ) -> DateTime<Local> {
        let mut rng = thread_rng();
        let jitter_second = jitter.unwrap_or(60) as i64;
        let target = match punch_type {
            PunchType::PunchIn => self
                .work_on_time
                .map(|t| {
                    t.checked_sub_signed(Duration::seconds(rng.gen_range(1..=jitter_second)))
                        .unwrap()
                })
                .unwrap(),
            PunchType::PunchOut => self
                .work_off_time
                .map(|t| {
                    t.checked_add_signed(Duration::seconds(rng.gen_range(1..=jitter_second)))
                        .unwrap()
                })
                .unwrap(),
        };
        target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    #[test]
    fn test_description() {
        let test_cases: [(WorkdaySchedule, &str); 3] = [
            (
                WorkdaySchedule {
                    date: "2023-01-01".to_string(),
                    work_on_time: None,
                    work_off_time: None,
                    memo: Some("元旦".to_string()),
                },
                "2023-01-01 休假日(元旦) N/A N/A",
            ),
            (
                WorkdaySchedule {
                    date: "2023-01-03".to_string(),
                    work_on_time: Some(Local.with_ymd_and_hms(2023, 1, 3, 09, 0, 0).unwrap()),
                    work_off_time: Some(Local.with_ymd_and_hms(2023, 1, 3, 18, 0, 0).unwrap()),
                    memo: None,
                },
                "2023-01-03 工作日 2023-01-03T09:00:00+08:00 2023-01-03T18:00:00+08:00",
            ),
            (
                WorkdaySchedule {
                    date: "2023-01-03".to_string(),
                    work_on_time: Some(Local.with_ymd_and_hms(2023, 1, 3, 09, 0, 0).unwrap()),
                    work_off_time: Some(Local.with_ymd_and_hms(2023, 1, 3, 18, 0, 0).unwrap()),
                    memo: Some("補班日".to_string()),
                },
                "2023-01-03 工作日(補班日) 2023-01-03T09:00:00+08:00 2023-01-03T18:00:00+08:00",
            ),
        ];

        for (w, s) in test_cases {
            assert_eq!(format!("{}", w), s);
        }
    }

    #[test]
    fn test_from_json_work_day() {
        let json = json!({
            "AdjustmentScheduleTime": true,
            "AdvanceLeave": true,
            "ArrangeLeave": true,
            "BillingStatus": 0,
            "CalendarEvent": {
              "CalendarEventId": "8d9ce128-b373-491d-a953-f59a378a5d59",
              "EventMemo": "國慶日補班",
              "EventStatus": 1,
              "ItemOptionId": "00002",
              "SubOptionId": null
            },
            "CycleSn": 6,
            "Date": "2023-09-23T00:00:00+00:00",
            "DayStartTime": "2023-09-22T18:00:00+00:00",
            "Employees": [],
            "FinalBilling": false,
            "IsAgreedWork": false,
            "IsEditable": false,
            "IsExistTransferShifts": false,
            "IsShiftRules": false,
            "ItemOptionId": "CY00001",
            "LeaveSheets": [],
            "OvertimeSheets": null,
            "PartialSupport": [],
            "SelectShiftSchedule": true,
            "ShiftId": "361068be-7113-46ec-a1fc-9c70d448dfdc",
            "ShiftSchedule": {
              "AgreedWorkEndTime": null,
              "AgreedWorkStartTime": null,
              "ColorCode": "#A654A3",
              "CycleSn": 1,
              "CycleStatus": 1,
              "IsWorkTimeChanged": false,
              "OriginalWorkOffTime": null,
              "OriginalWorkOnTime": null,
              "RestMinutes": 60.0,
              "ShiftScheduleId": "09709a43-7518-4ab5-b38f-09458408708b",
              "ShiftScheduleName": "正常0900",
              "ShiftScheduleRemark": "",
              "WorkOffTime": "2023-09-23T10:00:00+00:00",
              "WorkOnTime": "2023-09-23T01:00:00+00:00"
            },
            "SourceShiftScheduleName": "正常0900",
            "SourceShiftScheduleRemark": "",
            "SpecialEvents": [],
            "SupportDeptId": null,
            "SupportDeptName": null,
            "TripSheets": []
        });

        assert_eq!(
            format!("{}", WorkdaySchedule::from_json(&json)),
            "2023-09-23 工作日(國慶日補班) 2023-09-23T09:00:00+08:00 2023-09-23T18:00:00+08:00"
        );
    }

    #[test]
    fn test_from_json_holiday() {
        let json = json!({
            "AdjustmentScheduleTime": true,
            "AdvanceLeave": true,
            "ArrangeLeave": true,
            "BillingStatus": 0,
            "CalendarEvent": null,
            "CycleSn": 6,
            "Date": "2023-09-09T00:00:00+00:00",
            "DayStartTime": "2023-09-08T18:00:00+00:00",
            "Employees": [],
            "FinalBilling": false,
            "IsAgreedWork": false,
            "IsEditable": false,
            "IsExistTransferShifts": false,
            "IsShiftRules": false,
            "ItemOptionId": "CY00003",
            "LeaveSheets": [],
            "OvertimeSheets": null,
            "PartialSupport": [],
            "SelectShiftSchedule": true,
            "ShiftId": "361068be-7113-46ec-a1fc-9c70d448dfdc",
            "ShiftSchedule": {
              "AgreedWorkEndTime": null,
              "AgreedWorkStartTime": null,
              "ColorCode": "#A654A3",
              "CycleSn": 6,
              "CycleStatus": 2,
              "IsWorkTimeChanged": false,
              "OriginalWorkOffTime": null,
              "OriginalWorkOnTime": null,
              "RestMinutes": 60.0,
              "ShiftScheduleId": "09709a43-7518-4ab5-b38f-09458408708b",
              "ShiftScheduleName": "正常0900",
              "ShiftScheduleRemark": "",
              "WorkOffTime": null,
              "WorkOnTime": null
            },
            "SourceShiftScheduleName": "正常0900",
            "SourceShiftScheduleRemark": "",
            "SpecialEvents": [],
            "SupportDeptId": null,
            "SupportDeptName": null,
            "TripSheets": []
        });

        assert_eq!(
            format!("{}", WorkdaySchedule::from_json(&json)),
            "2023-09-09 休假日 N/A N/A"
        );
    }
}
