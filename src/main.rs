use clap::{crate_authors, crate_version, value_t_or_exit, App, Arg};
use colored::*;
use humantime;
use std::cmp::min;
use std::io;
use std::process::{Command, ExitStatus};
use std::thread::sleep;
use std::time::{Duration, Instant};

const MAX_ELAPSED: i32 = 4;
const UNABLE_TO_RUN: i32 = 3;
const KILLED_BY_SIGNAL: i32 = 2;
const MAX_RETRIES: i32 = 1;
const SUCCESS: i32 = 0;

fn main() {
    let matches = App::new("pll")
        .version(crate_version!())
        .author(crate_authors!())
        .about("pll - like watch but exits on success")
        .arg(Arg::with_name("command").required(true))
        .arg(
            Arg::with_name("initial-interval")
                .short("i")
                .long("initial-interval")
                .default_value("250ms")
                .help("initial wait time duration"),
        )
        .arg(
            Arg::with_name("max-interval")
                .long("max-interval")
                .default_value("2s")
                .help("max interval duration"),
        )
        .arg(
            Arg::with_name("max-elapsed")
                .short("m")
                .long("max-elapsed")
                .default_value("60s")
                .help("max elapsed duration"),
        )
        .arg(
            Arg::with_name("multiplier")
                .short("x")
                .long("multiplier")
                .default_value("1.3")
                .help("multiplier for interval"),
        )
        .arg(
            Arg::with_name("max-tries")
                .short("t")
                .long("max-tries")
                .default_value("45")
                .help("max tries total"),
        )
        .get_matches();

    let command = matches.value_of("command").unwrap();
    let initial_interval =
        value_t_or_exit!(matches, "initial-interval", humantime::Duration).into();
    let max_interval = value_t_or_exit!(matches, "max-interval", humantime::Duration).into();
    let max_elapsed = value_t_or_exit!(matches, "max-elapsed", humantime::Duration).into();
    let multiplier = value_t_or_exit!(matches, "multiplier", f32);
    let max_tries = value_t_or_exit!(matches, "max-tries", u32);

    let end_time = Instant::now() + max_elapsed;
    let exit = run_with_backoff(command, initial_interval, multiplier, max_interval, end_time, max_tries);
    ::std::process::exit(exit)
}

fn run_with_backoff(
    command: &str,
    wait: Duration,
    multiplier: f32,
    max_backoff: Duration,
    end_time: Instant,
    max_tries: u32,
) -> i32 {
    eprintln!("{}", format!("{}", command).white().bold());
    if Instant::now() > end_time {
        eprintln!("{}", format!("max elapsed, failing").red());
        MAX_ELAPSED
    } else {
        let status = run_in_current_shell(command);
        match status {
            Ok(e) => {
                match e.code() {
                    Some(code) if code == 0 => {
                        // success
                        eprintln!("{}", format!("success").green());
                        SUCCESS
                    }
                    Some(code) => {
                        // error
                        eprintln!(
                            "{}",
                            format!("exit code {}. retry after {}ms", code, wait.as_millis())
                                .yellow()
                        );
                        if max_tries > 0 {
                            sleep(wait);
                            let mut dur_ms = (wait.as_millis() as f32 * multiplier).floor();
                            dur_ms += jitter();

                            run_with_backoff(
                                command,
                                min(Duration::from_millis(dur_ms as u64), max_backoff),
                                multiplier,
                                max_backoff,
                                end_time,
                                max_tries - 1,
                            )
                        } else {
                            eprintln!("{}", format!("max retries, failing").red());
                            MAX_RETRIES
                        }
                    }
                    None => {
                        // killed
                        eprintln!("{}", format!("killed by signal").red());
                        KILLED_BY_SIGNAL
                    }
                }
            }
            Err(e) => {
                // unable to run
                eprintln!("{}", format!("unable to run command. {}", e).red());
                UNABLE_TO_RUN
            }
        }
    }
}

fn run_in_current_shell(command: &str) -> io::Result<ExitStatus> {
    let shell = match ::std::env::var("SHELL") {
        Ok(s) => s,
        Err(_) => "sh".to_owned(),
    };

    match shell.as_ref() {
        "bash" => Command::new("bash")
            .env_clear()
            .arg("-c")
            .arg(command)
            .status(),
        "sh" | _ => Command::new("sh")
            .env_clear()
            .arg("-c")
            .arg(command)
            .status(),
    }
}

fn jitter() -> f32 {
    2.0
}
