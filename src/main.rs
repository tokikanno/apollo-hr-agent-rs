mod apollo;

use std::fs::File;
use std::io::Write;
use std::process;

use crate::apollo::agent::{ApolloAgent, PunchType};
use apollo::utils::sleep_until;
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty};

#[derive(Parser, Debug)]
#[command(name = "apollo")]
#[command(author = "toki.kanno")]
struct Cli {
    #[arg(
        short,
        long,
        default_value = "config.json",
        help = "Config filename, you could skip the .json extension"
    )]
    config: String,
    #[command(subcommand)]
    command: SubCommands,
}

#[derive(Debug, Subcommand)]
enum SubCommands {
    #[command(about = "Initialize config file, the default config filename will be config.json")]
    Init {
        #[arg()]
        username: String,
        #[arg()]
        password: String,
        #[arg()]
        company: String,
    },

    #[command(about = "Auto punch by workday calendar setting")]
    AutoPunch {},

    #[command(about = "Punch in")]
    PunchIn {},

    #[command(about = "Punch out")]
    PunchOut {},

    #[command(about = "display worday calendar")]
    Calendar {},
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfigPayload {
    username: String,
    password: String,
    company: String,
}

fn get_config_filename(config_name: &String) -> String {
    return if config_name.ends_with(".json") {
        config_name.clone()
    } else {
        format!("{}.json", config_name)
    };
}

fn write_config_file(config_name: &String, username: &String, password: &String, company: &String) {
    let json = json!({
        "username": username,
        "password": password,
        "company": company,
    });
    let config_filename = get_config_filename(config_name);

    let mut file = File::create(config_filename).unwrap();
    file.write_all(to_string_pretty(&json).unwrap().as_bytes())
        .unwrap();
}

fn prepare_agent(config_name: &String) -> Result<ApolloAgent, String> {
    let config_filename = get_config_filename(config_name);
    let file = File::open(&config_filename).map_err(|e| {
        format!(
            r#"can't open {}
reason: {}

if this is your first time usage, try call init subcommand first,
        "#,
            &config_filename, e
        )
    })?;
    let config: ConfigPayload = serde_json::from_reader(file)
        .map_err(|e| format!("can't parse {} into json.\nreason: {}", &config_filename, e))?;

    let mut agent = ApolloAgent::new(config.username, config.password, config.company);
    agent.login()?;

    Ok(agent)
}

fn print_calendars(agent: &ApolloAgent) {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let schedules = agent.get_workday_schedules(None, None).unwrap();
    for s in schedules {
        println!(
            "{}{}",
            s,
            if s.get_date() == today {
                " <-- today"
            } else {
                ""
            }
        );
    }
}

fn _do_auto_punch(agent: &mut ApolloAgent) {
    // always re-login
    agent.login().unwrap();

    let schedule = agent.get_today_schedule().unwrap();

    println!("{}", schedule);

    if !schedule.is_work_day() {
        println!("{} is not work day", schedule.get_date());
        return;
    }

    let punch_in_time = schedule.get_punch_time_with_jitter(PunchType::PunchIn, None);
    let punch_out_time = schedule.get_punch_time_with_jitter(PunchType::PunchOut, None);

    println!(
        r#"Auto punch time arranged:
    punch in : {}
    punch out: {}
    "#,
        punch_in_time, punch_out_time
    );

    let mut now = Local::now();
    if now < punch_in_time {
        sleep_until(&punch_in_time);
        _do_punch(agent, PunchType::PunchIn);
    } else {
        println!(
            "punch in skipped, because current time has exceeded the scheduled auto punch time"
        )
    }

    now = Local::now();
    if now < punch_out_time {
        sleep_until(&punch_out_time);
        _do_punch(agent, PunchType::PunchOut);
    } else {
        println!(
            "punch out skipped, because current time has exceeded the scheduled auto punch time"
        )
    }
}

fn _do_punch(agent: &mut ApolloAgent, punch_type: PunchType) {
    match agent.punch_card(punch_type) {
        Ok(v) => print!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => println!("{}", e),
    }
}

fn auto_punch(agent: &mut ApolloAgent) {
    loop {
        _do_auto_punch(agent);

        sleep_until(
            &Local
                .from_local_datetime(
                    &Local::now()
                        .date_naive()
                        .succ_opt()
                        .unwrap()
                        .and_hms_opt(7, 0, 0)
                        .unwrap(),
                )
                .unwrap(),
        )
    }
}

fn main() {
    let args = Cli::parse();

    match args.command {
        SubCommands::Init {
            username,
            password,
            company,
        } => write_config_file(&args.config, &username, &password, &company),

        _ => {
            let mut agent = match prepare_agent(&args.config) {
                Ok(v) => v,
                Err(e) => {
                    println!("{}", e);
                    process::exit(-1);
                }
            };

            match args.command {
                SubCommands::AutoPunch {} => auto_punch(&mut agent),
                SubCommands::PunchIn {} => _do_punch(&mut agent, PunchType::PunchIn),
                SubCommands::PunchOut {} => _do_punch(&mut agent, PunchType::PunchOut),
                SubCommands::Calendar {} => print_calendars(&agent),
                _ => {
                    unreachable!("You should not pass!!!")
                }
            }
        }
    }
}
