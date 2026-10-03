//! Data structures and parsing for AI agent windows.

/// Summary of a tmux window parsed for agent detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowSummary {
    pub id: String,
    pub name: String,
    pub is_agent: bool,
}

/// Parses the output of `tmux list-windows -F '#{window_id}\t#{window_name}\t#{@mox_is_agent}'`.
pub fn parse_window_list_for_agents(raw: &str) -> Vec<WindowSummary> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 2 {
                return None;
            }
            let id = parts[0].to_string();
            let name = parts[1].to_string();
            let is_agent = parts.get(2).map(|s| s.trim() == "1").unwrap_or(false);
            Some(WindowSummary { id, name, is_agent })
        })
        .collect()
}

/// Calculates the next AI agent window name (`agent`, `agent-2`, `agent-3`, etc.).
pub fn calculate_next_agent_name(existing_names: &[String]) -> String {
    if !existing_names.iter().any(|n| n == "agent") {
        return "agent".to_string();
    }

    let mut counter = 2;
    loop {
        let candidate = format!("agent-{counter}");
        if !existing_names.iter().any(|n| n == &candidate) {
            return candidate;
        }
        counter += 1;
    }
}
