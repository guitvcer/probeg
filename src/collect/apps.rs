//! Installed apps and translation of bundle identifiers into human-readable names.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Local};

use crate::model::{Household, Usage};

const SYSTEM_DIRS: [&str; 3] = [
    "/System/Applications",
    "/System/Applications/Utilities",
    "/System/Library/CoreServices",
];
const BREW_ROOTS: [&str; 2] = ["/opt/homebrew", "/usr/local"];

#[derive(Default)]
pub struct Installed {
    pub user: Vec<App>,
    pub names: HashMap<String, String>,
    pub system_count: u64,
    pub brew_formulae: Option<u64>,
    pub brew_casks: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct App {
    pub name: String,
    pub bundle: Option<String>,
    pub added: Option<DateTime<Local>>,
}

pub fn scan() -> Installed {
    let mut installed = Installed::default();
    for dir in user_dirs() {
        for path in app_bundles(&dir, 1) {
            let app = read_app(&path);
            if let Some(bundle) = &app.bundle {
                installed.names.insert(bundle.clone(), app.name.clone());
            }
            installed.user.push(app);
        }
    }
    for dir in SYSTEM_DIRS {
        for path in app_bundles(Path::new(dir), 0) {
            let app = read_app(&path);
            if dir != SYSTEM_DIRS[2] {
                installed.system_count += 1;
            }
            if let Some(bundle) = app.bundle {
                installed.names.insert(bundle, app.name);
            }
        }
    }
    for root in BREW_ROOTS {
        let root = Path::new(root);
        if root.join("Cellar").is_dir() {
            installed.brew_formulae = dir_size(&root.join("Cellar"));
            installed.brew_casks = dir_size(&root.join("Caskroom"));
            break;
        }
    }
    installed
}

fn user_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    dirs
}

fn dir_size(path: &Path) -> Option<u64> {
    Some(
        std::fs::read_dir(path)
            .ok()?
            .flatten()
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .count() as u64,
    )
}

/// Apps in a folder and, down to `depth`, in subfolders such as `Utilities`.
fn app_bundles(dir: &Path, depth: u32) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "app") {
            found.push(path);
        } else if depth > 0 && path.is_dir() {
            found.extend(app_bundles(&path, depth - 1));
        }
    }
    found
}

fn read_app(path: &Path) -> App {
    let info = plist::Value::from_file(path.join("Contents/Info.plist")).ok();
    let dict = info.as_ref().and_then(plist::Value::as_dictionary);
    let text = |key: &str| {
        dict.and_then(|d| d.get(key))
            .and_then(plist::Value::as_string)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    App {
        name: text("CFBundleDisplayName")
            .or_else(|| text("CFBundleName"))
            .unwrap_or(stem),
        bundle: text("CFBundleIdentifier"),
        added: std::fs::metadata(path)
            .and_then(|m| m.created())
            .ok()
            .map(DateTime::<Local>::from),
    }
}

/// Fallback for an unknown identifier: com.foo.BarApp → BarApp.
fn fallback_name(bundle: &str) -> String {
    bundle.rsplit('.').next().unwrap_or(bundle).to_string()
}

pub fn name_usage(usage: &mut Usage, installed: &Installed) {
    let name = |bundle: &str| {
        installed
            .names
            .get(bundle)
            .cloned()
            .unwrap_or_else(|| fallback_name(bundle))
    };
    for app in &mut usage.apps {
        app.name = name(&app.bundle);
    }
    if let Some(longest) = &mut usage.longest_focus {
        longest.name = name(&longest.bundle);
    }
}

pub fn fill(household: &mut Household, installed: &Installed, usage: &Usage) {
    household.apps_installed = Some(installed.user.len() as u64);
    household.apps_system = Some(installed.system_count);
    household.brew_formulae = installed.brew_formulae;
    household.brew_casks = installed.brew_casks;
    household.apps_unopened = (!usage.apps.is_empty()).then(|| unopened(&installed.user, usage));
    let newest = installed
        .user
        .iter()
        .filter(|a| a.added.is_some())
        .max_by_key(|a| a.added);
    household.newest_app = newest.map(|a| a.name.clone());
    household.newest_app_date = newest.and_then(|a| a.added);
    let month_ago = Local::now() - Duration::days(30);
    household.apps_new_30d = Some(
        installed
            .user
            .iter()
            .filter(|a| a.added.is_some_and(|d| d > month_ago))
            .count() as u64,
    );
}

fn unopened(apps: &[App], usage: &Usage) -> u64 {
    let opened: HashSet<&str> = usage.apps.iter().map(|a| a.bundle.as_str()).collect();
    apps.iter()
        .filter(|a| {
            a.bundle
                .as_deref()
                .is_none_or(|bundle| !opened.contains(bundle))
        })
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppUse;

    fn app(bundle: &str) -> App {
        App {
            name: bundle.into(),
            bundle: Some(bundle.into()),
            added: None,
        }
    }

    fn used(bundle: &str) -> AppUse {
        AppUse {
            bundle: bundle.into(),
            name: bundle.into(),
            secs: 1.0,
        }
    }

    #[test]
    fn unopened_apps_are_those_missing_from_the_journal() {
        let usage = Usage {
            apps: vec![used("a")],
            ..Usage::default()
        };
        assert_eq!(unopened(&[app("a"), app("b"), app("c")], &usage), 2);
    }

    #[test]
    fn unknown_bundles_get_a_readable_name() {
        let mut installed = Installed::default();
        installed
            .names
            .insert("company.thebrowser.Browser".into(), "Arc".into());
        let mut usage = Usage {
            apps: vec![
                used("company.thebrowser.Browser"),
                used("com.apple.Whatever"),
            ],
            longest_focus: Some(used("com.apple.Whatever")),
            ..Usage::default()
        };
        name_usage(&mut usage, &installed);
        assert_eq!(usage.apps[0].name, "Arc");
        assert_eq!(usage.apps[1].name, "Whatever");
        assert_eq!(usage.longest_focus.unwrap().name, "Whatever");
    }

    #[test]
    fn real_applications_folder_scans_without_panic() {
        let installed = scan();
        assert!(installed.system_count > 0, "every Mac has system apps");
    }
}
