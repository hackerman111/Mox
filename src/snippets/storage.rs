//! Persistent storage for terminal command snippets in TOML format.

use super::model::Snippet;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnippetsDocument {
    #[serde(default, alias = "snippet")]
    pub snippets: Vec<Snippet>,
}

/// Returns the default snippets path ($XDG_CONFIG_HOME/mox/snippets.toml or ~/.config/mox/snippets.toml).
pub fn default_snippets_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.trim().is_empty()
    {
        return Some(PathBuf::from(xdg.trim()).join("mox").join("snippets.toml"));
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Some(
            PathBuf::from(home.trim())
                .join(".config")
                .join("mox")
                .join("snippets.toml"),
        );
    }
    crate::state::directory().ok().map(|d| d.join("snippets.toml"))
}

/// Built-in starter snippets.
pub fn default_snippets() -> Vec<Snippet> {
    vec![
        Snippet {
            id: "docker-exec".into(),
            title: "Docker Container Shell".into(),
            command: "docker exec -it <container_id> <shell:bash>".into(),
            description: "Open interactive shell inside running Docker container".into(),
            tags: vec!["docker".into(), "container".into(), "shell".into()],
        },
        Snippet {
            id: "git-new-branch".into(),
            title: "Git Create & Switch Branch".into(),
            command: "git checkout -b <branch>".into(),
            description: "Create and immediately switch to a new branch".into(),
            tags: vec!["git".into(), "branch".into()],
        },
        Snippet {
            id: "git-commit".into(),
            title: "Git Commit with Message".into(),
            command: "git commit -m \"<message>\"".into(),
            description: "Record staged repository changes with a message".into(),
            tags: vec!["git".into(), "commit".into()],
        },
        Snippet {
            id: "tmux-new-session".into(),
            title: "Tmux New Named Session".into(),
            command: "tmux new -s <session_name> -c <directory:~>".into(),
            description: "Start a new named tmux session in specific directory".into(),
            tags: vec!["tmux".into(), "session".into()],
        },
        Snippet {
            id: "curl-post-json".into(),
            title: "cURL POST JSON".into(),
            command: "curl -X POST <url> -H \"Content-Type: application/json\" -d '<body:{}>'".into(),
            description: "Send HTTP POST request with JSON payload".into(),
            tags: vec!["curl".into(), "http".into(), "api".into()],
        },
        Snippet {
            id: "ripgrep-search".into(),
            title: "Ripgrep Search in Directory".into(),
            command: "rg \"<query>\" <path:.>".into(),
            description: "Fast recursive text search across directory".into(),
            tags: vec!["ripgrep".into(), "search".into(), "find".into()],
        },
    ]
}

pub fn serialize_snippets(snippets: &[Snippet]) -> Result<String, String> {
    let doc = SnippetsDocument {
        snippets: snippets.to_vec(),
    };
    toml::to_string_pretty(&doc).map_err(|e| format!("Failed to serialize snippets to TOML: {e}"))
}

pub fn deserialize_snippets(content: &str) -> Result<Vec<Snippet>, String> {
    let doc: SnippetsDocument =
        toml::from_str(content).map_err(|e| format!("Failed to parse snippets TOML: {e}"))?;
    Ok(doc.snippets)
}

pub fn load_snippets(path: Option<&Path>) -> Vec<Snippet> {
    let resolved = path.map(Path::to_path_buf).or_else(default_snippets_path);
    let Some(file_path) = resolved else {
        return default_snippets();
    };
    if !file_path.exists() {
        return default_snippets();
    }
    match std::fs::read_to_string(&file_path) {
        Ok(content) => match deserialize_snippets(&content) {
            Ok(snippets) => {
                if snippets.is_empty() {
                    default_snippets()
                } else {
                    snippets
                }
            }
            Err(err) => {
                eprintln!(
                    "Warning: failed to parse snippets file '{}': {err}",
                    file_path.display()
                );
                default_snippets()
            }
        },
        Err(_) => default_snippets(),
    }
}

pub fn save_snippets(path: Option<&Path>, snippets: &[Snippet]) -> Result<(), String> {
    let resolved = path
        .map(Path::to_path_buf)
        .or_else(default_snippets_path)
        .ok_or("Cannot resolve snippets file path")?;
    let toml_str = serialize_snippets(snippets)?;
    crate::state::atomic_write(&resolved, toml_str.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_snippets_and_serialization() {
        let defaults = default_snippets();
        assert!(!defaults.is_empty());
        let toml_str = serialize_snippets(&defaults).expect("serializes to toml");
        let parsed = deserialize_snippets(&toml_str).expect("deserializes from toml");
        assert_eq!(parsed.len(), defaults.len());
        assert_eq!(parsed[0].title, defaults[0].title);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_file = std::env::temp_dir().join("mox_test_snippets.toml");
        let _ = std::fs::remove_file(&temp_file);

        let custom = vec![Snippet {
            id: "test-cmd".into(),
            title: "Test Custom Command".into(),
            command: "echo <text:hello>".into(),
            description: "Echo test".into(),
            tags: vec!["test".into()],
        }];

        save_snippets(Some(&temp_file), &custom).expect("saves successfully");
        let loaded = load_snippets(Some(&temp_file));
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-cmd");

        let _ = std::fs::remove_file(&temp_file);
    }
}
