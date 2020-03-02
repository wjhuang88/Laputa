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

    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            let level = match record.level() {
                Error => Error.to_string().as_str().bright_red().on_black(),
                Warn => Warn.to_string().as_str().bright_yellow().on_black(),
                Info => Info.to_string().as_str().bright_green().on_black(),
                Debug => Debug.to_string().as_str().bright_cyan().on_black(),
                Trace => Trace.to_string().as_str().white().on_black(),
            };
            writeln!(
                buf,
                "{} [{:<5}] {} \t-- [module: {}][thread: {}]",
                Local::now()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .as_str()
                    .bright_black(),
                level,
                &record.args(),
                record
                    .module_path()
                    .unwrap_or("<unknown>")
                    .bright_black()
                    .italic(),
                std::thread::current()
                    .name()
                    .unwrap_or("<unknown>")
                    .bright_black()
                    .italic(),
            )
        })
        .init();

    info!("Logger config initialized");
}
