//! What a check found, and how it is printed (text for people, `--json` for scripts).

use craft_app_descriptor::Report;
use serde::Serialize;

/// One package found in the release, as `--json` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PackageOut {
    pub name: String,
    pub version: String,
    /// `<os>-<arch>`.
    pub target: String,
    pub suffix: String,
}

/// The result of `dir` or `github`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub ok: bool,
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
    pub packages: Vec<PackageOut>,
}

impl Outcome {
    /// Adds the contract's report to what the tool itself found.
    pub fn absorb(&mut self, report: Report) {
        self.problems.extend(report.problems);
        self.warnings.extend(report.warnings);
        self.packages.extend(report.packages.into_iter().map(|p| PackageOut {
            name: p.name,
            version: p.version,
            target: p.token,
            suffix: p.suffix.to_string(),
        }));
    }

    /// Settles `ok` once every check has run.
    pub fn finish(mut self) -> Self {
        self.ok = self.problems.is_empty();
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"ok\":false,\"problems\":[\"couldn't write JSON: {e}\"]}}"))
    }

    /// The text report. `annotate` adds GitHub Actions `::error::` / `::warning::` lines so the
    /// problems show up on the workflow run's summary.
    pub fn to_text(&self, title: &str, annotate: bool) -> String {
        let mut out = format!("{title}\n");
        if !self.packages.is_empty() {
            out += "packages:\n";
            for p in &self.packages {
                out += &format!("  {}  ({})\n", p.name, p.target);
            }
        }
        for w in &self.warnings {
            out += &format!("warning: {w}\n");
        }
        for p in &self.problems {
            out += &format!("problem: {p}\n");
        }
        if annotate {
            for w in &self.warnings {
                out += &format!("::warning title=craft-release-check::{}\n", escape_annotation(w));
            }
            for p in &self.problems {
                out += &format!("::error title=craft-release-check::{}\n", escape_annotation(p));
            }
        }
        out += &match self.problems.len() {
            0 => format!("OK: the release keeps the craft-app.json contract ({} warning(s))\n", self.warnings.len()),
            n => format!("FAILED: {n} problem(s), {} warning(s)\n", self.warnings.len()),
        };
        out
    }
}

/// GitHub's workflow-command escaping for a message.
fn escape_annotation(s: &str) -> String {
    s.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_json_agree() {
        let o = Outcome { problems: vec!["bad 100%".into()], warnings: vec!["meh".into()], ..Outcome::default() }.finish();
        assert!(!o.ok);
        let text = o.to_text("t", true);
        assert!(text.contains("problem: bad 100%"));
        assert!(text.contains("::error title=craft-release-check::bad 100%25"));
        assert!(text.ends_with("FAILED: 1 problem(s), 1 warning(s)\n"));
        let v: serde_json::Value = serde_json::from_str(&o.to_json()).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["warnings"][0], "meh");
        assert!(v["packages"].as_array().unwrap().is_empty());
    }
}
