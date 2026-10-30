//! Command-line parsing, by hand: four subcommands and a handful of `--flag value` options don't
//! need a dependency.

use std::path::PathBuf;

pub const USAGE: &str = "\
craft-release-check: checks a release against the craft-app.json release contract

USAGE:
    craft-release-check dir <dist-dir> --repo <owner/name> --tag <vX.Y.Z> [--json]
        Check a local release folder before upload: craft-app.json, the file names, the icon,
        and SHA256SUMS.txt (every listed checksum is recomputed).
    craft-release-check github --repo <owner/name> [--tag <vX.Y.Z>] [--json]
        Check a published GitHub release (the latest one without --tag). GITHUB_TOKEN, when set,
        is sent to api.github.com only.
    craft-release-check sums <dist-dir>
        Write <dist-dir>/SHA256SUMS.txt for every file in the folder.
    craft-release-check init --repo <owner/name> --id <id> --name <Name>
        Print a starter craft-app.json to edit.

EXIT CODES:
    0  the release keeps the contract (warnings may be printed)
    1  the release breaks the contract
    2  the check couldn't run (bad arguments, unreadable folder, network error)
";

/// What to do, read from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Dir { dist: PathBuf, repo: String, tag: String, json: bool },
    Github { repo: String, tag: Option<String>, json: bool },
    Sums { dist: PathBuf },
    Init { repo: String, id: String, name: String },
    Help,
    Version,
}

#[derive(Default)]
struct Raw {
    positional: Vec<String>,
    repo: Option<String>,
    tag: Option<String>,
    id: Option<String>,
    name: Option<String>,
    json: bool,
}

/// Reads the arguments after the program name.
pub fn parse_args<I, S>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut it = args.into_iter().map(Into::into);
    let Some(sub) = it.next() else { return Ok(Command::Help) };
    match sub.as_str() {
        "-h" | "--help" | "help" => return Ok(Command::Help),
        "-V" | "--version" => return Ok(Command::Version),
        _ => {}
    }
    let mut raw = Raw::default();
    while let Some(arg) = it.next() {
        if arg == "-h" || arg == "--help" {
            return Ok(Command::Help);
        }
        if arg == "--json" {
            raw.json = true;
            continue;
        }
        let Some(flag) = arg.strip_prefix("--") else {
            raw.positional.push(arg);
            continue;
        };
        let (key, value) = match flag.split_once('=') {
            Some((k, v)) => (k.to_string(), v.to_string()),
            None => (flag.to_string(), it.next().ok_or_else(|| format!("--{flag} needs a value"))?),
        };
        let slot = match key.as_str() {
            "repo" => &mut raw.repo,
            "tag" => &mut raw.tag,
            "id" => &mut raw.id,
            "name" => &mut raw.name,
            _ => return Err(format!("unknown option --{key}")),
        };
        if slot.replace(value).is_some() {
            return Err(format!("--{key} is given twice"));
        }
    }
    build(&sub, raw)
}

fn build(sub: &str, mut raw: Raw) -> Result<Command, String> {
    let allowed: &[&str] = match sub {
        "dir" => &["repo", "tag", "json"],
        "github" => &["repo", "tag", "json"],
        "sums" => &[],
        "init" => &["repo", "id", "name"],
        _ => return Err(format!("unknown command {sub:?} (try --help)")),
    };
    for (key, given) in
        [("repo", raw.repo.is_some()), ("tag", raw.tag.is_some()), ("id", raw.id.is_some()), ("name", raw.name.is_some()), ("json", raw.json)]
    {
        if given && !allowed.contains(&key) {
            return Err(format!("{sub} doesn't take --{key}"));
        }
    }
    let wants_dir = matches!(sub, "dir" | "sums");
    let dist = if wants_dir {
        match raw.positional.len() {
            1 => raw.positional.pop().map(PathBuf::from),
            0 => return Err(format!("{sub} needs a <dist-dir>")),
            _ => return Err(format!("{sub} takes one <dist-dir>, got {}", raw.positional.len())),
        }
    } else {
        if let Some(extra) = raw.positional.first() {
            return Err(format!("{sub} doesn't take {extra:?}"));
        }
        None
    };
    let need = |v: Option<String>, key: &str| v.filter(|s| !s.is_empty()).ok_or_else(|| format!("{sub} needs --{key}"));
    Ok(match (sub, dist) {
        ("dir", Some(dist)) => Command::Dir { dist, repo: need(raw.repo, "repo")?, tag: need(raw.tag, "tag")?, json: raw.json },
        ("github", _) => Command::Github { repo: need(raw.repo, "repo")?, tag: raw.tag.filter(|t| !t.is_empty()), json: raw.json },
        ("sums", Some(dist)) => Command::Sums { dist },
        ("init", _) => Command::Init { repo: need(raw.repo, "repo")?, id: need(raw.id, "id")?, name: need(raw.name, "name")? },
        _ => return Err(format!("unknown command {sub:?} (try --help)")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Command, String> {
        parse_args(s.split_whitespace())
    }

    #[test]
    fn subcommands_read() {
        assert_eq!(
            parse("dir dist --repo o/n --tag v1.0.0 --json"),
            Ok(Command::Dir { dist: "dist".into(), repo: "o/n".into(), tag: "v1.0.0".into(), json: true })
        );
        assert_eq!(
            parse("dir --repo=o/n dist --tag=1.0.0"),
            Ok(Command::Dir { dist: "dist".into(), repo: "o/n".into(), tag: "1.0.0".into(), json: false })
        );
        assert_eq!(parse("github --repo o/n"), Ok(Command::Github { repo: "o/n".into(), tag: None, json: false }));
        assert_eq!(
            parse("github --repo o/n --tag v2.0.0 --json"),
            Ok(Command::Github { repo: "o/n".into(), tag: Some("v2.0.0".into()), json: true })
        );
        assert_eq!(parse("sums out"), Ok(Command::Sums { dist: "out".into() }));
        assert_eq!(parse("init --repo o/n --id x --name X"), Ok(Command::Init { repo: "o/n".into(), id: "x".into(), name: "X".into() }));
        assert_eq!(parse(""), Ok(Command::Help));
        assert_eq!(parse("dir --help"), Ok(Command::Help));
        assert_eq!(parse("--version"), Ok(Command::Version));
    }

    #[test]
    fn mistakes_are_named() {
        for (args, says) in [
            ("dir dist --tag v1", "needs --repo"),
            ("dir dist --repo o/n", "needs --tag"),
            ("dir --repo o/n --tag v1", "needs a <dist-dir>"),
            ("dir a b --repo o/n --tag v1", "takes one"),
            ("dir dist --repo o/n --repo p/q --tag v1", "given twice"),
            ("dir dist --repo o/n --tag", "needs a value"),
            ("dir dist --repo o/n --tag v1 --colour red", "unknown option"),
            ("sums dist --json", "doesn't take --json"),
            ("github --repo o/n extra", "doesn't take"),
            ("init --repo o/n --id x", "needs --name"),
            ("publish", "unknown command"),
        ] {
            let e = parse(args).unwrap_err();
            assert!(e.contains(says), "{args}: {e}");
        }
    }
}
