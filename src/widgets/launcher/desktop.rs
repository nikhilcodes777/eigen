use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub description: String,
    pub exec: String,
    pub icon: String,
    pub categories: Vec<String>,
}

/// Scan standard XDG directories for `.desktop` files and parse them.
pub fn load_desktop_entries() -> Vec<DesktopEntry> {
    let mut dirs: Vec<PathBuf> = vec![PathBuf::from("/usr/share/applications")];

    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(format!(
            "{home}/.local/share/applications"
        )));
    }

    if let Ok(xdg) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg.split(':') {
            let p = PathBuf::from(dir).join("applications");
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        }
    }

    let mut entries = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for dir in dirs {
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(de) = parse_desktop_file(&path)
                && seen_names.insert(de.name.clone()) {
                    entries.push(de);
                }
        }
    }

    entries.sort_by_key(|a| a.name.to_lowercase());
    entries
}

fn parse_desktop_file(path: &PathBuf) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = String::new();
    let mut comment = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut categories = Vec::new();
    let mut no_display = false;
    let mut is_app = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            // We've left the [Desktop Entry] section
            if in_desktop_entry {
                break;
            }
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Name" => name = value.to_string(),
                "Comment" => comment = value.to_string(),
                "Exec" => exec = value.to_string(),
                "Icon" => icon = value.to_string(),
                "Categories" => {
                    categories = value.split(';').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
                }
                "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                "Type" => is_app = value == "Application",
                _ => {}
            }
        }
    }

    if name.is_empty() || exec.is_empty() || no_display || !is_app {
        return None;
    }

    // Strip field codes from Exec (%u, %U, %f, %F, etc.)
    let exec_clean = exec
        .split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    Some(DesktopEntry {
        name,
        description: comment,
        exec: exec_clean,
        icon,
        categories,
    })
}
