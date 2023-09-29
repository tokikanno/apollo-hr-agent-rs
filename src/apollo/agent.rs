use super::workday_schedule::WorkdaySchedule;
use crate::apollo::utils::to_resp_json;
use chrono::{Datelike, Local};
use reqwest;
use serde_json::{json, Value};
use std::fmt::Display;
use visdom::Vis;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";

pub enum PunchType {
    PunchIn = 1,
    PunchOut = 2,
}

impl Display for PunchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PunchType::PunchIn => write!(f, "PunchIn"),
            PunchType::PunchOut => write!(f, "PunchOut"),
        }
    }
}

pub struct ApolloAgent {
    username: String,
    password: String,
    company: String,

    client: reqwest::blocking::Client,

    auth_data: Option<Value>,
}

impl ApolloAgent {
    pub fn new<S: Into<String>>(username: S, password: S, company: S) -> Self {
        ApolloAgent {
            username: username.into(),
            password: password.into(),
            company: company.into(),
            client: reqwest::blocking::Client::builder()
                .user_agent(USER_AGENT)
                .cookie_store(true)
                .build()
                .unwrap(),
            auth_data: None,
        }
    }

    pub fn login(&mut self) -> Result<(), String> {
        let auth_data = self.get_login_req_token()?;

        self.check_ticket(auth_data["code"].as_str().unwrap())?;
        self.get_authorized()?;

        // keep success auth data
        self.auth_data = Some(auth_data);

        Ok(())
    }

    fn do_api_request(&self, builder: reqwest::blocking::RequestBuilder) -> Result<Value, String> {
        let resp = builder.send().map_err(|err| err.to_string())?;
        to_resp_json(resp)
    }

    fn do_html_request(
        &self,
        builder: reqwest::blocking::RequestBuilder,
    ) -> Result<String, String> {
        let resp = builder.send().map_err(|err| err.to_string())?;
        resp.text().map_err(|err| err.to_string())
    }

    pub fn get_login_req_token(&self) -> Result<Value, String> {
        let html = self.do_html_request(
            self.client
                .get("https://asiaauth.mayohr.com/HRM/Account/Login"),
        )?;

        let token = Vis::load(html)
            .map_err(|err| err.to_string())?
            .find(r#"input[name="__RequestVerificationToken"]"#)
            .first()
            .val()
            .to_string();

        let payload: &[(&str, &str)] = &[
            ("__RequestVerificationToken", &token),
            ("companyCode", &self.company),
            ("employeeNo", &self.username),
            ("grant_type", "password"),
            ("locale", "zh-tw"),
            ("password", &self.password),
            ("red", "https,//apollo.mayohr.com/tube"),
            ("userName", &format!("{}-{}", self.company, self.username)),
        ];

        self.do_api_request(
            self.client
                .post("https://asiaauth.mayohr.com/Token")
                .form(payload),
        )
    }

    pub fn check_ticket(&self, auth_code: &str) -> Result<Value, String> {
        self.do_api_request(
            self.client
                .get("https://linkup-be.mayohr.com/api/auth/checkticket")
                .query(&[("code", auth_code)]),
        )
    }

    pub fn get_authorized(&self) -> Result<Value, String> {
        self.do_api_request(
            self.client
                .get("https://linkup-be.mayohr.com/api/Authorization/GetAuthorized"),
        )
    }

    pub fn get_employee_calendars(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<Value, String> {
        let now = Local::now();

        self.do_api_request(
            self.client
                .get("https://pt-be.mayohr.com/api/EmployeeCalendars/scheduling")
                .header("Functioncode", "PersonalShiftSchedule")
                .header("Actioncode", "Default")
                .query(&[
                    ("year", year.unwrap_or(now.year()).to_string().as_str()),
                    ("month", month.unwrap_or(now.month()).to_string().as_str()),
                ]),
        )
    }

    pub fn get_workday_schedules(
        &self,
        year: Option<i32>,
        month: Option<u32>,
    ) -> Result<Vec<WorkdaySchedule>, String> {
        let resp = self.get_employee_calendars(year, month)?;
        let calendars = resp["Data"]["Calendars"]
            .as_array()
            .ok_or_else(|| "No .Data.Calendars found in response".to_string())?;

        let schedules: Vec<WorkdaySchedule> =
            calendars.iter().map(WorkdaySchedule::from_json).collect();

        Ok(schedules)
    }

    pub fn get_today_schedule(&self) -> Result<WorkdaySchedule, String> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let schedules = self.get_workday_schedules(None, None)?;

        schedules
            .into_iter()
            .find(|x| x.get_date() == today)
            .ok_or_else(|| format!("Can not find WorkdaySchedule of {}", today))
    }

    pub fn punch_card(&self, punch_type: PunchType) -> Result<Value, String> {
        self.do_api_request(
            self.client
                .post("https://pt-be.mayohr.com/api/checkIn/punch/web")
                .header("Functioncode", "PunchCard")
                .header("Actioncode", "Default")
                .json(&json!({
                    "AttendanceType": punch_type as u8,
                    "IsOverride": false,
                })),
        )
    }
}
