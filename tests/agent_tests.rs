use mox::agent::{calculate_next_agent_name, parse_window_list_for_agents};

#[test]
fn test_calculate_next_agent_name() {
    let existing = vec!["editor".to_string(), "bash".to_string()];
    assert_eq!(calculate_next_agent_name(&existing), "agent");

    let with_agent = vec!["editor".to_string(), "agent".to_string()];
    assert_eq!(calculate_next_agent_name(&with_agent), "agent-2");

    let with_agent_2 = vec![
        "editor".to_string(),
        "agent".to_string(),
        "agent-2".to_string(),
    ];
    assert_eq!(calculate_next_agent_name(&with_agent_2), "agent-3");

    // Non-consecutive agent numbers: should find lowest available or next max
    let gaps = vec!["agent".to_string(), "agent-5".to_string()];
    assert_eq!(calculate_next_agent_name(&gaps), "agent-2");
}

#[test]
fn test_parse_window_list_for_agents() {
    let raw = "@1\teditor\t0\n@2\tagent\t1\n@3\tzsh\t\n";
    let windows = parse_window_list_for_agents(raw);
    assert_eq!(windows.len(), 3);
    assert_eq!(windows[0].id, "@1");
    assert_eq!(windows[0].name, "editor");
    assert!(!windows[0].is_agent);

    assert_eq!(windows[1].id, "@2");
    assert_eq!(windows[1].name, "agent");
    assert!(windows[1].is_agent);

    assert_eq!(windows[2].id, "@3");
    assert_eq!(windows[2].name, "zsh");
    assert!(!windows[2].is_agent);
}
