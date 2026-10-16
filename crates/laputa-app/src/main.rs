use std::{env, ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};
use laputa_media::{FfprobeMediaProbe, ProjectStore};

fn main() -> Result<()> {
    run(env::args_os().skip(1).collect())
}

fn run(args: Vec<OsString>) -> Result<()> {
    let Some(command) = args.first().and_then(|value| value.to_str()) else {
        print_usage();
        return Ok(());
    };

    match command {
        "create" => {
            if args.len() != 3 {
                bail!("usage: laputa-app create <project-dir> <project-name>");
            }
            let root = PathBuf::from(&args[1]);
            let name = args[2].to_string_lossy();
            let project = ProjectStore::at(root).create(&name)?;
            println!("{}", serde_json::to_string_pretty(&project)?);
        }
        "import" => {
            if args.len() < 3 {
                bail!("usage: laputa-app import <project-dir> <media>...");
            }
            let root = PathBuf::from(&args[1]);
            let sources = args[2..].iter().map(PathBuf::from).collect::<Vec<_>>();
            let report = ProjectStore::at(root)
                .import_media(&sources, &FfprobeMediaProbe)
                .context("media import failed; project originals were not modified")?;
            println!("imported {} asset(s)", report.imported.len());
            println!("{}", serde_json::to_string_pretty(&report.project)?);
        }
        "open" => {
            if args.len() != 2 {
                bail!("usage: laputa-app open <project-dir>");
            }
            let project = ProjectStore::at(PathBuf::from(&args[1])).open()?;
            println!("{}", serde_json::to_string_pretty(&project)?);
        }
        "help" | "--help" | "-h" => print_usage(),
        other => bail!("unknown command {other:?}; run laputa-app --help"),
    }

    Ok(())
}

fn print_usage() {
    println!(
        "Laputa foundation commands:\n  laputa-app create <project-dir> <project-name>\n  laputa-app import <project-dir> <media>...\n  laputa-app open <project-dir>"
    );
}
