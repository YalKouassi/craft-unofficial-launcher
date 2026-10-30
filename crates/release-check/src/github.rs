//! `github`: a published release, read through GitHub's REST API.
//!
//! Everything GitHub returns is treated like the launcher treats it: size-capped before it is
//! read, and only fetched from `api.github.com` for the repo that was asked about. A
//! `GITHUB_TOKEN`, when set, goes to `api.github.com` only; ureq drops it on the redirect to the
//! asset CDN.

use std::io::Read;
use std::time::Duration;

use craft_app_descriptor::descriptor::valid_repo;
use craft_app_descriptor::{
    DESCRIPTOR_FILE, Descriptor, MAX_DESCRIPTOR_BYTES, MAX_ICON_BYTES, SUMS_FILE, check_icon, check_release, parse_sha256sums,
};
use serde::Deserialize;

use crate::outcome::Outcome;
use crate::{MAX_SUMS_BYTES, sums};

/// Generic, with no personal details (AGENTS.md).
pub const USER_AGENT: &str = concat!("craft-release-check/", env!("CARGO_PKG_VERSION"));
const API: &str = "https://api.github.com";
/// A release's JSON (it lists every asset) larger than this is refused.
const MAX_RELEASE_JSON: usize = 8 * 1024 * 1024;

/// The parts of GitHub's release object this tool reads.
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    /// The API URL (`…/releases/assets/<id>`), fetched with `Accept: application/octet-stream`.
    pub url: String,
    #[serde(default)]
    pub size: u64,
}

/// Fetches the release (`tag`, or the latest) of `repo` and checks it.
pub fn check_github(repo: &str, tag: Option<&str>) -> Result<(String, Outcome), String> {
    if !valid_repo(repo) {
        return Err(format!("--repo {repo:?} must be a GitHub owner/name"));
    }
    let client = Client::new();
    let url = match tag {
        Some(t) => format!("{API}/repos/{repo}/releases/tags/{}", encode(t)),
        None => format!("{API}/repos/{repo}/releases/latest"),
    };
    let body = client.get(&url, "application/vnd.github+json", MAX_RELEASE_JSON).map_err(|e| match e {
        Fetch::NotFound => match tag {
            Some(t) => format!("{repo} has no published release tagged {t} (a private repo needs GITHUB_TOKEN)"),
            None => format!("{repo} has no published release (a private repo needs GITHUB_TOKEN)"),
        },
        Fetch::Other(e) => e,
    })?;
    let release: Release = serde_json::from_slice(&body).map_err(|e| format!("GitHub's release answer isn't what was expected: {e}"))?;
    let prefix = format!("{API}/repos/{repo}/releases/assets/");
    let outcome = evaluate(repo, &release, |asset, max| {
        if !asset.url.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()) {
            return Err(format!("{}'s download URL isn't on {prefix}", asset.name));
        }
        client.get(&asset.url, "application/octet-stream", max).map_err(|e| match e {
            Fetch::NotFound => format!("{} couldn't be downloaded (404)", asset.name),
            Fetch::Other(e) => format!("{} couldn't be downloaded: {e}", asset.name),
        })
    })?;
    Ok((release.tag_name, outcome))
}

/// Checks `release` of `repo`, downloading the descriptor, sums and icon through `fetch(asset,
/// max_bytes)`. `Err` means a download failed and the check couldn't finish.
pub fn evaluate<F>(repo: &str, release: &Release, mut fetch: F) -> Result<Outcome, String>
where
    F: FnMut(&Asset, usize) -> Result<Vec<u8>, String>,
{
    let mut out = Outcome::default();
    let find = |name: &str| release.assets.iter().find(|a| a.name == name);
    if release.draft {
        out.warnings.push("this release is a draft: the launcher can't see it until it's published".into());
    }
    if release.prerelease {
        out.warnings.push("this release is marked pre-release, so GitHub's \"latest release\" skips it".into());
    }

    let mut capped = |asset: &Asset, max: usize, out: &mut Outcome| -> Result<Option<Vec<u8>>, String> {
        if asset.size > max as u64 {
            out.problems.push(format!("{} is {} bytes; the limit is {max}", asset.name, asset.size));
            return Ok(None);
        }
        fetch(asset, max).map(Some)
    };

    let sums_text = match find(SUMS_FILE) {
        Some(a) => match capped(a, MAX_SUMS_BYTES, &mut out)? {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(t) => Some(t),
                Err(_) => {
                    out.problems.push(format!("{SUMS_FILE} isn't UTF-8 text"));
                    None
                }
            },
            None => None,
        },
        None => None,
    };
    if let Some(n) = sums_text.as_deref().map(sums::malformed_lines).filter(|n| *n > 0) {
        out.warnings.push(format!("{SUMS_FILE} has {n} line(s) that aren't `<sha256>  <file name>`; the launcher ignores them"));
    }
    // A downloaded file must match every line that lists it.
    let verify = |name: &str, bytes: &[u8], out: &mut Outcome| {
        let Some(text) = &sums_text else { return };
        let got = sums::sha256_bytes(bytes);
        for (want, _) in parse_sha256sums(text).into_iter().filter(|(_, n)| n == name) {
            if want != got {
                out.problems.push(format!("{name}: its SHA-256 is {got}, but {SUMS_FILE} says {want}"));
            }
        }
    };

    let descriptor = match find(DESCRIPTOR_FILE) {
        // One byte past the cap, so Descriptor::parse reports an oversized file as such.
        Some(a) => match capped(a, MAX_DESCRIPTOR_BYTES.saturating_add(1), &mut out)? {
            Some(bytes) => {
                verify(DESCRIPTOR_FILE, &bytes, &mut out);
                match Descriptor::parse(&bytes, repo) {
                    Ok(d) => Some(d),
                    Err(p) => {
                        out.problems.extend(p.0);
                        None
                    }
                }
            }
            None => None,
        },
        None => {
            out.problems.push(format!("{DESCRIPTOR_FILE} isn't attached"));
            None
        }
    };
    let Some(d) = descriptor else {
        out.problems.push("the release contract wasn't checked, because craft-app.json couldn't be read".into());
        return Ok(out.finish());
    };

    if let Some(a) = d.icon.as_deref().and_then(find)
        && let Some(bytes) = capped(a, MAX_ICON_BYTES.saturating_add(1), &mut out)?
    {
        verify(&a.name, &bytes, &mut out);
        if let Err(e) = check_icon(&bytes) {
            out.problems.push(format!("{}: {e}", a.name));
        }
    }

    let names: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
    out.absorb(check_release(&d, &release.tag_name, &names, sums_text.as_deref()));
    Ok(out.finish())
}

/// Percent-encodes a tag for a URL path segment.
fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

enum Fetch {
    NotFound,
    Other(String),
}

struct Client {
    agent: ureq::Agent,
    token: Option<String>,
}

impl Client {
    fn new() -> Self {
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(Duration::from_secs(120)))
            .http_status_as_error(false)
            .user_agent(USER_AGENT)
            .build()
            .new_agent();
        let token = std::env::var("GITHUB_TOKEN").ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        Self { agent, token }
    }

    /// GETs `url`, refusing an answer whose decoded body is larger than `max`.
    fn get(&self, url: &str, accept: &str, max: usize) -> Result<Vec<u8>, Fetch> {
        let mut req = self.agent.get(url).header("Accept", accept).header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token
            && url.starts_with(API)
        {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
        let mut resp = req.call().map_err(|e| Fetch::Other(format!("{url}: {e}")))?;
        match resp.status().as_u16() {
            200 => {}
            404 => return Err(Fetch::NotFound),
            403 | 429 => {
                return Err(Fetch::Other(format!(
                    "GitHub refused {url} (HTTP {}): probably its rate limit; set GITHUB_TOKEN",
                    resp.status().as_u16()
                )));
            }
            s => return Err(Fetch::Other(format!("{url}: HTTP {s}"))),
        }
        // ureq's own limit sits below the gzip decoder, so cap the decoded stream instead.
        let mut bytes = Vec::new();
        resp.body_mut()
            .with_config()
            .limit(u64::MAX)
            .reader()
            .take((max as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| Fetch::Other(format!("{url}: {e}")))?;
        if bytes.len() > max {
            return Err(Fetch::Other(format!("{url}: the answer is larger than {max} bytes")));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESC: &str = r##"{"schema":1,"id":"notecraft","name":"NoteCraft","tagline":"Notes","repo":"o/notecraft","publisher":"O",
        "bundle_id":"org.o.notecraft","color":"#112233"}"##;

    fn release(names: &[&str]) -> Release {
        Release {
            tag_name: "v1.0.0".into(),
            draft: false,
            prerelease: false,
            assets: names
                .iter()
                .enumerate()
                .map(|(i, n)| Asset { name: n.to_string(), url: format!("{API}/repos/o/notecraft/releases/assets/{i}"), size: 10 })
                .collect(),
        }
    }

    fn files(sums_for_desc: &str) -> impl FnMut(&Asset, usize) -> Result<Vec<u8>, String> {
        let pkg = "notecraft-1.0.0-linux-x86_64.tar.gz";
        let sums = format!("{sums_for_desc}  {DESCRIPTOR_FILE}\n{}  {pkg}\n", "0".repeat(64));
        move |a: &Asset, _| match a.name.as_str() {
            DESCRIPTOR_FILE => Ok(DESC.as_bytes().to_vec()),
            SUMS_FILE => Ok(sums.clone().into_bytes()),
            other => Err(format!("unexpected download of {other}")),
        }
    }

    #[test]
    fn a_good_release_passes_and_only_small_files_are_fetched() {
        let r = release(&["notecraft-1.0.0-linux-x86_64.tar.gz", DESCRIPTOR_FILE, SUMS_FILE]);
        let o = evaluate("o/notecraft", &r, files(&sums::sha256_bytes(DESC.as_bytes()))).unwrap();
        assert!(o.ok, "{:?}", o.problems);
        assert_eq!(o.packages.len(), 1);
    }

    #[test]
    fn a_descriptor_that_doesnt_match_its_checksum_fails() {
        let r = release(&["notecraft-1.0.0-linux-x86_64.tar.gz", DESCRIPTOR_FILE, SUMS_FILE]);
        let o = evaluate("o/notecraft", &r, files(&"f".repeat(64))).unwrap();
        assert!(o.problems.iter().any(|p| p.starts_with("craft-app.json: its SHA-256")), "{:?}", o.problems);
    }

    #[test]
    fn oversized_assets_are_never_downloaded() {
        let mut r = release(&["notecraft-1.0.0-linux-x86_64.tar.gz", DESCRIPTOR_FILE, SUMS_FILE]);
        for a in &mut r.assets {
            a.size = 1 << 30;
        }
        let o = evaluate("o/notecraft", &r, |a, _| Err(format!("fetched {}", a.name))).unwrap();
        assert!(!o.ok);
        assert!(o.problems.iter().any(|p| p.contains("the limit is")), "{:?}", o.problems);
    }

    #[test]
    fn tags_are_encoded() {
        assert_eq!(encode("v1.0.0+build/x"), "v1.0.0%2Bbuild%2Fx");
    }
}
