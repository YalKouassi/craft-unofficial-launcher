//! Menu entries (freedesktop `.desktop` files) for AppImages the launcher installs.
//!
//! An AppImage installs nothing by itself, so without an entry the app is missing from the
//! desktop's menu and, on Wayland, the dock can't match the window's app id to an icon. The
//! launcher writes `<data>/applications/<bundle id>.desktop` (the id each app's window
//! reports) and marks it as its own, so it only ever replaces or removes entries it wrote.
//!
//! The text is built by pure functions, tested on every platform; only Linux writes the files.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::path::Path;

use crate::App;

/// The key that marks an entry as written by the launcher.
pub const MARKER: &str = "X-Craft-Unofficial-Launcher=true";

/// The menu categories every entry the launcher writes declares. Craft-style apps are creative
/// tools; an app's own `.desktop` file (written by its AppImage) can be more specific.
pub const CATEGORIES: &str = "Graphics;";

/// `path` as one quoted argument of an `Exec=` key, per the Desktop Entry spec: inside double
/// quotes `"`, `` ` ``, `$` and `\` take a backslash, then the string-value rule doubles every
/// backslash, and `%` (field codes) is written `%%`. `None` for paths that can't be written
/// safely (not UTF-8, holding a control character such as a line break, or holding `=`, which the
/// spec forbids in the executable name or path even when quoted).
pub fn exec_quote(path: &Path) -> Option<String> {
    let s = path.to_str()?;
    if s.chars().any(|c| c.is_control() || c == '=') {
        return None;
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' | '`' | '$' => {
                out.push_str("\\\\");
                out.push(c);
            }
            '\\' => out.push_str("\\\\\\\\"),
            '%' => out.push_str("%%"),
            c => out.push(c),
        }
    }
    out.push('"');
    Some(out)
}

/// A localestring/string value: backslashes and line breaks escaped.
fn value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t")
}

/// The entry for `app` installed as the AppImage `appimage`; `icon` when its icon is installed
/// under the app's id. `None` when the path can't be put in an `Exec=` line.
pub fn render(app: &App, appimage: &Path, icon: bool) -> Option<String> {
    let exec = exec_quote(appimage)?;
    let mut text = String::from("[Desktop Entry]\nType=Application\n");
    text.push_str(&format!("Name={}\n", value(app.name)));
    text.push_str(&format!("Comment={}\n", value(app.tagline)));
    text.push_str(&format!("Exec={exec} %F\n"));
    if icon {
        text.push_str(&format!("Icon={}\n", app.bundle_id()));
    }
    text.push_str("Terminal=false\n");
    text.push_str(&format!("Categories={CATEGORIES}\n"));
    text.push_str(&format!("StartupWMClass={}\n", app.bundle_id()));
    text.push_str(MARKER);
    text.push('\n');
    Some(text)
}

/// Whether the launcher wrote this entry.
pub fn is_ours(text: &str) -> bool {
    text.lines().any(|l| l.trim() == MARKER)
}

/// The program an entry's `Exec=` runs: its first argument, unquoted. Reads the launcher's own
/// quoting and the plain `Exec="<path>" %F` the apps' AppImages write for themselves.
pub fn exec_program(text: &str) -> Option<String> {
    let line = text.lines().find_map(|l| l.strip_prefix("Exec="))?;
    // String-value escapes first, then the quoting rule.
    let mut unescaped = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => unescaped.push(' '),
                Some('n') => unescaped.push('\n'),
                Some('t') => unescaped.push('\t'),
                Some('r') => unescaped.push('\r'),
                Some(other) => unescaped.push(other),
                None => unescaped.push('\\'),
            }
        } else {
            unescaped.push(c);
        }
    }
    let mut out = String::new();
    let mut chars = unescaped.trim_start().chars();
    if unescaped.trim_start().starts_with('"') {
        chars.next();
        loop {
            match chars.next()? {
                '"' => break,
                '\\' => out.push(chars.next()?),
                c => out.push(c),
            }
        }
    } else {
        out.extend(chars.take_while(|c| !c.is_whitespace()));
    }
    Some(out.replace("%%", "%"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn app(id: &str) -> &'static App {
        crate::testing::app(id)
    }

    #[test]
    fn writes_a_complete_entry() {
        let text = render(app("examplecraft"), Path::new("/home/ana/Applications/examplecraft-0.6.0-linux-x86_64.AppImage"), true).unwrap();
        let expected = format!(
            "[Desktop Entry]\nType=Application\nName=ExampleCraft\nComment={}\n\
             Exec=\"/home/ana/Applications/examplecraft-0.6.0-linux-x86_64.AppImage\" %F\n\
             Icon=org.example.examplecraft\nTerminal=false\nCategories=Graphics;\n\
             StartupWMClass=org.example.examplecraft\nX-Craft-Unofficial-Launcher=true\n",
            app("examplecraft").tagline
        );
        assert_eq!(text, expected);
        assert!(is_ours(&text));
        let no_icon = render(app("samplecraft"), Path::new("/x/w.AppImage"), false).unwrap();
        assert!(!no_icon.contains("Icon="));
        assert!(no_icon.contains("StartupWMClass=org.example.samplecraft\n"));
        assert!(no_icon.contains("Categories=Graphics;\n"));
    }

    #[test]
    fn quotes_exec_paths_per_the_spec() {
        assert_eq!(exec_quote(Path::new("/home/ana/My Apps/p.AppImage")).unwrap(), "\"/home/ana/My Apps/p.AppImage\"");
        assert_eq!(exec_quote(Path::new("/a/\"q\" $HOME `x`.AppImage")).unwrap(), r#""/a/\\"q\\" \\$HOME \\`x\\`.AppImage""#);
        assert_eq!(exec_quote(Path::new(r"/a\b/100%.AppImage")).unwrap(), r#""/a\\\\b/100%%.AppImage""#);
        assert_eq!(exec_quote(Path::new("/a/line\nbreak.AppImage")), None);
        // The spec forbids `=` in the executable name or path, even quoted.
        assert_eq!(exec_quote(Path::new("/opt/artcraft=beta/app")), None);
        assert!(render(app("examplecraft"), Path::new("/a/line\nbreak.AppImage"), true).is_none());
    }

    #[test]
    fn reads_back_the_program_it_wrote_and_the_apps_own_entries() {
        for p in ["/home/ana/My Apps/p.AppImage", "/a/\"q\" $HOME `x`.AppImage", r"/a\b/100%.AppImage", "/plain.AppImage"] {
            let text = render(app("samplecraft"), Path::new(p), false).unwrap();
            assert_eq!(exec_program(&text).map(PathBuf::from), Some(PathBuf::from(p)), "{p}");
        }
        // ExampleCraft's AppRun writes its own entry, unmarked.
        let theirs = "[Desktop Entry]\nName=ExampleCraft\nExec=\"/home/ana/Applications/examplecraft-0.5.0-linux-x86_64.AppImage\" %F\n";
        assert_eq!(exec_program(theirs).as_deref(), Some("/home/ana/Applications/examplecraft-0.5.0-linux-x86_64.AppImage"));
        assert!(!is_ours(theirs));
        assert_eq!(exec_program("[Desktop Entry]\nExec=examplecraft %F\n").as_deref(), Some("examplecraft"));
        assert_eq!(exec_program("[Desktop Entry]\nName=x\n"), None);
    }
}
