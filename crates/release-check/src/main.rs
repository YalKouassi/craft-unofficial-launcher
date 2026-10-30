//! `craft-release-check`: see `--help` and docs/craft-app-spec.md.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;

use craft_app_descriptor::SUMS_FILE;
use craft_release_check::args::USAGE;
use craft_release_check::{Command, Outcome, dir, github, init, parse_args, sums};

fn main() -> ExitCode {
    let command = match parse_args(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("craft-release-check: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("craft-release-check: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> Result<ExitCode, String> {
    match command {
        Command::Help => {
            print!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("craft-release-check {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        Command::Dir { dist, repo, tag, json } => {
            let outcome = dir::check_dir(&dist, &repo, &tag)?;
            Ok(report(&outcome, &format!("craft-release-check: {} as {repo} {tag}", dist.display()), json))
        }
        Command::Github { repo, tag, json } => {
            let (tag, outcome) = github::check_github(&repo, tag.as_deref())?;
            Ok(report(&outcome, &format!("craft-release-check: https://github.com/{repo}/releases/tag/{tag}"), json))
        }
        Command::Sums { dist } => {
            let n = sums::write_sums(&dist)?;
            println!("wrote {} ({n} file(s))", dist.join(SUMS_FILE).display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Init { repo, id, name } => {
            let d = init::starter(&repo, &id, &name)?;
            print!("{}", d.to_json());
            eprintln!("{}", init::NOTE);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn report(outcome: &Outcome, title: &str, json: bool) -> ExitCode {
    if json {
        println!("{}", outcome.to_json());
    } else {
        let annotate = std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true");
        print!("{}", outcome.to_text(title, annotate));
    }
    if outcome.ok { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
