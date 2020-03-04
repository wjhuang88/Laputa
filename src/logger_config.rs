use chrono;
use env_logger;
use log::info;

use colored::Colorize;

pub(crate) const BANNER: &'static str = r"
      __   Castle in the Sky  __
     / /   ____ _____  __  __/ /_____ _
    / /   / __ `/ __ \/ / / / __/ __ `/
   / /___/ /_/ / /_/ / /_/ / /_/ /_/ /
  /_____/\__,_/ .___/\__,_/\__/\__,_/
             /_/         ❤ by wj.huang
";

pub fn print_banner() {
    println!("{}", BANNER.bright_blue());
}

pub fn init_log() {
    use chrono::Local;
    use colored::*;
    use log::Level::*;
    use std::io::Write;

    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug");
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            let level = match record.level() {
                Error => Error.to_string().as_str().bright_red(),
                Warn => Warn.to_string().as_str().bright_yellow(),
                Info => Info.to_string().as_str().bright_green(),
                Debug => Debug.to_string().as_str().bright_cyan(),
                Trace => Trace.to_string().as_str().white(),
            };
            let date_time = Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .as_str()
                .bright_black();
            let module = {
                let module = record.module_path().unwrap_or("<unknown>");
                let len = module.len();
                if len > 20 {
                    &module[(len - 20)..]
                } else {
                    module
                }
            }
            .bright_cyan()
            .italic();
            let args = &record.args();
            let current = std::thread::current();
            let thread = {
                let thread = current.name().unwrap_or("<unknown>");
                let len = thread.len();
                if len > 8 {
                    &thread[(len - 8)..]
                } else {
                    thread
                }
            }
            .bright_black()
            .italic();
            writeln!(
                buf,
                "{time} [{level:<5}] --- [{thread:>8}] {module:<20} : {args}",
                time = date_time,
                level = level,
                module = module,
                args = args,
                thread = thread,
            )
        })
        .init();

    info!("Logger config initialized");
}
