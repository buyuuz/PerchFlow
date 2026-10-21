//! Bounded, opt-in inspection of registered Python entrypoint string literals.
//! Only the .py source is read (max 128 KiB), NEVER Excel/TXT/JSON file bodies.
//! Candidates are hints, not a complete dependency graph.
use crate::config::{self, model::{ProjectDef, TaskDef}};
use serde::Serialize;
use std::{collections::HashSet, fs, io::Read, path::{Path,PathBuf}, time::UNIX_EPOCH};

const MAX_SOURCE: u64 = 128 * 1024;
const MAX_CANDIDATES: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct LinkedFile {
    pub path: String,
    pub resolved_path: String,
    pub source: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub modified_unix: Option<u64>,
    pub inferred: bool,
}

pub fn is_safe_document(path: &Path) -> bool {
    matches!(path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase().as_str(),
        "xlsx"|"xls"|"xlsm"|"csv"|"txt"|"json"|"jsonl"|"ini"|"yaml"|"yml"|"xml"|"md")
}

fn safe_document_path(project: &ProjectDef, raw: &str) -> Option<PathBuf> {
    if raw.len() > 512 || raw.is_empty() || raw.chars().any(|c| c.is_control()) {
        return None;
    }
    let cleaned = raw.replace("\\\\", "\\"); // Python string with escaped backslashes
    if cleaned.starts_with("\\\\") || cleaned.starts_with("//") {
        return None; // no UNC/network targets
    }
    let path = Path::new(&cleaned);
    let candidate = if path.is_absolute() {
        if cleaned.split(|c| c == '/' || c == '\\').any(|p| p == "..") { return None; }
        path.to_path_buf()
    } else {
        if !config::safe_relative_path(&cleaned) { return None; }
        Path::new(&project.root).join(&cleaned)
    };
    if !is_safe_document(&candidate) { return None; }
    Some(candidate)
}

fn entry_script(project: &ProjectDef, task: &TaskDef) -> Option<PathBuf> {
    let script = task.args.first().map(Path::new)?;
    if script.extension()?.to_str()?.to_ascii_lowercase() != "py" { return None; }
    let root = Path::new(&project.root).canonicalize().ok()?;
    let real = script.canonicalize().ok()?;
    if !real.starts_with(&root) { return None; }
    if fs::symlink_metadata(&real).ok()?.is_file() { Some(real) } else { None }
}

/// Extract Python single/double quoted literals; no code execution, imports
/// or evaluation. Escape handling only decodes escaped backslashes/quotes.
fn string_literals(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut result = Vec::new();
    let mut index = 0;
    while index < chars.len() && result.len() < 512 {
        let quote = chars[index];
        if quote != '\'' && quote != '"' { index += 1; continue; }
        // Ignore docstrings by rejecting triple quote openers.
        if index + 2 < chars.len() && chars[index+1] == quote && chars[index+2] == quote {
            index += 3;
            while index + 2 < chars.len() {
                if chars[index] == quote && chars[index+1] == quote && chars[index+2] == quote { index += 3; break; }
                index += 1;
            }
            continue;
        }
        let raw = index > 0 && matches!(chars[index-1], 'r'|'R') &&
            (index < 2 || !chars[index-2].is_alphanumeric());
        index += 1;
        let mut literal = String::new();
        let mut ended = false;
        while index < chars.len() && literal.len() < 512 {
            let ch = chars[index];
            if ch == quote { index += 1; ended = true; break; }
            if ch == '\\' && index + 1 < chars.len() {
                let next = chars[index+1];
                if !raw && (next == '\\' || next == quote) {
                    literal.push(next); index += 2; continue;
                }
                literal.push(ch); index += 1; continue;
            }
            if ch == '\n' || ch == '\r' { break; }
            literal.push(ch); index += 1;
        }
        if ended { result.push(literal); }
    }
    result
}

pub fn discover(project: &ProjectDef, task: &TaskDef) -> Vec<LinkedFile> {
    let mut found = Vec::new();
    let mut unique = HashSet::new();
    let mut add = |raw: &str, source: &str| {
        if found.len() >= MAX_CANDIDATES { return; }
        let Some(path) = safe_document_path(project, raw) else { return };
        let display = path.to_string_lossy().to_string();
        if !unique.insert(display.to_lowercase()) { return; }
        let info = fs::symlink_metadata(&path).ok().filter(|m|m.is_file() && !m.file_type().is_symlink());
        // Do not report a symlink target as openable.
        let safe_info = info.filter(|_| path.canonicalize().is_ok());
        found.push(LinkedFile {
            path: raw.to_owned(),
            resolved_path: display,
            source: source.to_owned(),
            exists: safe_info.is_some(),
            size_bytes: safe_info.as_ref().map(|m|m.len()),
            modified_unix: safe_info.as_ref().and_then(|m|m.modified().ok())
                .and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs()),
            inferred: source == "code_candidate",
        });
    };
    // Explicit project paths do not require reading any project file.
    for input in &project.inputs { add(input, "registered_input"); }
    for output in &project.outputs { add(output, "registered_output"); }
    if let Some(script) = entry_script(project, task) {
        if let Ok(file) = fs::File::open(script) {
            let mut bytes = Vec::new();
            if file.take(MAX_SOURCE).read_to_end(&mut bytes).is_ok() {
                let contents = String::from_utf8_lossy(&bytes);
                for literal in string_literals(&contents) { add(&literal, "code_candidate"); }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoted_python_paths_discovered_without_execution() {
        let text = r#"from pathlib import Path
EXCEL = Path("tasks.xlsx")
BACKUP = r"D:\Docs\report.csv"
PASSWORD = "secret.txt"
def foo(): return open("settings.json")
"#;
        let values = string_literals(text);
        assert!(values.iter().any(|s|s == "tasks.xlsx"));
        assert!(values.iter().any(|s|s == r"D:\Docs\report.csv"));
        assert!(!values.iter().any(|s|s == "from pathlib import Path"));
    }
    #[test]
    fn blocks_network_and_executable_file_types() {
        let p = ProjectDef {name:"p".into(),root:"/tmp/project".into(),tasks:vec![],
            inputs:vec![],outputs:vec![],output_dirs:vec![],preflight_inputs:false};
        assert!(safe_document_path(&p, r"\\server\share\data.xlsx").is_none());
        assert!(safe_document_path(&p,"../private.txt").is_none());
        assert!(safe_document_path(&p,"launch.exe").is_none());
        assert!(safe_document_path(&p,"files/data.csv").is_some());
    }
}
