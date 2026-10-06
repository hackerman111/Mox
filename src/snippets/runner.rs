//! Interactive TUI runner and fuzzy search for snippets.

use super::model::Snippet;
use super::storage::{load_snippets, save_snippets};
use super::template::{TemplateParam, extract_parameters, substitute_parameters};
use super::ui::render_snippets_view;
use crate::tui::RawModeGuard;
use crate::ui::Viewport;
use crate::ui::theme::Theme;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{cursor, execute};
use std::io::stdout;
use std::time::Duration;

pub fn filter_snippets(snippets: &[Snippet], query: &str) -> Vec<Snippet> {
    if query.trim().is_empty() {
        return snippets.to_vec();
    }
    let query_lower = query.trim().to_lowercase();
    let mut scored: Vec<(Snippet, i64)> = snippets
        .iter()
        .filter_map(|s| {
            let searchable = format!(
                "{} {} {} {}",
                s.title,
                s.tags.join(" "),
                s.description,
                s.command
            );
            crate::ui::fuzzy::fuzzy_match(&searchable, &query_lower).map(|score| (s.clone(), score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.title.cmp(&b.0.title)));
    scored.into_iter().map(|(s, _)| s).collect()
}

#[derive(Debug, PartialEq, Eq)]
enum AddStep {
    Title,
    Command,
    Description,
    Tags,
}

enum AppMode {
    Normal,
    Search,
    ConfirmDelete,
    PromptParam {
        snippet: Snippet,
        param_index: usize,
        params: Vec<TemplateParam>,
        collected: Vec<(String, String)>,
        buf: String,
        copy_only: bool,
    },
    AddWizard {
        step: AddStep,
        title: String,
        command: String,
        description: String,
        tags: String,
        buf: String,
    },
}

pub fn run_snippets_panel(socket: Option<&str>) -> Result<(), String> {
    let theme = Theme::load(socket);
    let _guard = RawModeGuard::new()?;
    let mut out = std::io::BufWriter::new(stdout());
    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

    let mut all_snippets = load_snippets(None);
    let mut query = String::new();
    let mut selected = 0usize;
    let mut offset = 0usize;
    let mut mode = AppMode::Normal;

    let mut needs_redraw = true;
    let mut last_size = (0usize, 0usize);

    loop {
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let (term_w, term_h) = (term_w as usize, term_h as usize);

        if (term_w, term_h) != last_size {
            last_size = (term_w, term_h);
            needs_redraw = true;
            let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
        }

        let filtered = filter_snippets(&all_snippets, &query);
        let list_len = filtered.len();
        if list_len > 0 {
            selected = selected.min(list_len - 1);
            let content_h = term_h.saturating_sub(4);
            offset = Viewport::adjust_scroll(selected, offset, content_h);
        } else {
            selected = 0;
            offset = 0;
        }

        if needs_redraw {
            let confirm_str = if let AppMode::ConfirmDelete = &mode {
                filtered.get(selected).map(|s| s.title.as_str())
            } else {
                None
            };

            let prompt_data = match &mode {
                AppMode::PromptParam {
                    params,
                    param_index,
                    buf,
                    ..
                } => {
                    if let Some(param) = params.get(*param_index) {
                        let hint = match &param.default {
                            Some(def) => format!("Param <{}> (default: {def})", param.name),
                            None => format!("Param <{}>", param.name),
                        };
                        Some((hint, buf.clone()))
                    } else {
                        None
                    }
                }
                AppMode::AddWizard { step, buf, .. } => {
                    let label = match step {
                        AddStep::Title => "New Snippet Title",
                        AddStep::Command => "Command Template (e.g. docker exec -it <id> sh)",
                        AddStep::Description => "Description",
                        AddStep::Tags => "Tags (comma separated)",
                    };
                    Some((label.to_string(), buf.clone()))
                }
                _ => None,
            };

            let prompt_ref = prompt_data.as_ref().map(|(t, b)| (t.as_str(), b.as_str()));

            let view_state = super::ui::SnippetsViewState {
                snippets: &filtered,
                query: &query,
                selected,
                offset,
                confirm_delete: confirm_str,
                prompt_input: prompt_ref,
                width: term_w,
                height: term_h,
            };
            render_snippets_view(&mut out, &view_state, &theme)?;
            needs_redraw = false;
        }

        if !event::poll(Duration::from_millis(200)).map_err(|e| e.to_string())? {
            continue;
        }
        let Event::Key(key) = event::read().map_err(|e| e.to_string())? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        needs_redraw = true;

        match &mut mode {
            AppMode::Normal => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Char('/') => mode = AppMode::Search,
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') if list_len > 0 => {
                    selected = (selected + 1).min(list_len - 1);
                }
                KeyCode::PageUp => {
                    selected = selected.saturating_sub(10);
                }
                KeyCode::PageDown if list_len > 0 => {
                    selected = (selected + 10).min(list_len - 1);
                }
                KeyCode::Char('d') if !filtered.is_empty() => {
                    mode = AppMode::ConfirmDelete;
                }
                KeyCode::Char('a') => {
                    mode = AppMode::AddWizard {
                        step: AddStep::Title,
                        title: String::new(),
                        command: String::new(),
                        description: String::new(),
                        tags: String::new(),
                        buf: String::new(),
                    };
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    if let Some(snippet) = filtered.get(selected).cloned() {
                        let copy_only = key.code == KeyCode::Char('y');
                        let params = extract_parameters(&snippet.command);
                        if params.is_empty() {
                            let cmd = snippet.command.clone();
                            if copy_only {
                                crate::clipboard::copy(socket, &cmd)?;
                            } else {
                                execute_command_in_pane(socket, &cmd)?;
                            }
                            break;
                        } else {
                            mode = AppMode::PromptParam {
                                snippet,
                                param_index: 0,
                                params,
                                collected: Vec::new(),
                                buf: String::new(),
                                copy_only,
                            };
                        }
                    }
                }
                _ => {}
            },
            AppMode::Search => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    mode = AppMode::Normal;
                }
                KeyCode::Backspace => {
                    query.pop();
                    selected = 0;
                    offset = 0;
                }
                KeyCode::Up => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down if list_len > 0 => {
                    selected = (selected + 1).min(list_len - 1);
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query.push(c);
                    selected = 0;
                    offset = 0;
                }
                _ => {}
            },
            AppMode::ConfirmDelete => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    if let Some(target) = filtered.get(selected) {
                        all_snippets.retain(|s| s.id != target.id);
                        let _ = save_snippets(None, &all_snippets);
                    }
                    mode = AppMode::Normal;
                }
                _ => {
                    mode = AppMode::Normal;
                }
            },
            AppMode::PromptParam {
                snippet,
                param_index,
                params,
                collected,
                buf,
                copy_only,
            } => match key.code {
                KeyCode::Esc => {
                    mode = AppMode::Normal;
                }
                KeyCode::Backspace => {
                    buf.pop();
                }
                KeyCode::Enter => {
                    let current_param = &params[*param_index];
                    let val = if buf.trim().is_empty() {
                        current_param.default.clone().unwrap_or_default()
                    } else {
                        buf.trim().to_string()
                    };
                    collected.push((current_param.name.clone(), val));
                    *param_index += 1;
                    buf.clear();

                    if *param_index >= params.len() {
                        let final_cmd = substitute_parameters(&snippet.command, collected);
                        if *copy_only {
                            crate::clipboard::copy(socket, &final_cmd)?;
                        } else {
                            execute_command_in_pane(socket, &final_cmd)?;
                        }
                        break;
                    }
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buf.push(c);
                }
                _ => {}
            },
            AppMode::AddWizard {
                step,
                title,
                command,
                description,
                tags,
                buf,
            } => match key.code {
                KeyCode::Esc => {
                    mode = AppMode::Normal;
                }
                KeyCode::Backspace => {
                    buf.pop();
                }
                KeyCode::Enter => {
                    let val = buf.trim().to_string();
                    buf.clear();
                    match step {
                        AddStep::Title => {
                            *title = val;
                            *step = AddStep::Command;
                        }
                        AddStep::Command => {
                            *command = val;
                            *step = AddStep::Description;
                        }
                        AddStep::Description => {
                            *description = val;
                            *step = AddStep::Tags;
                        }
                        AddStep::Tags => {
                            *tags = val;
                            let parsed_tags = tags
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            let new_id = title
                                .to_lowercase()
                                .chars()
                                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                                .collect::<String>();
                            let new_snippet = Snippet {
                                id: if new_id.is_empty() {
                                    "custom-snippet".into()
                                } else {
                                    new_id
                                },
                                title: title.clone(),
                                command: command.clone(),
                                description: description.clone(),
                                tags: parsed_tags,
                            };
                            all_snippets.push(new_snippet);
                            let _ = save_snippets(None, &all_snippets);
                            mode = AppMode::Normal;
                        }
                    }
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buf.push(c);
                }
                _ => {}
            },
        }
    }

    Ok(())
}

fn execute_command_in_pane(socket: Option<&str>, command: &str) -> Result<(), String> {
    crate::tmux::execute_tmux(socket, &["send-keys", "-l", command])?;
    crate::tmux::execute_tmux(socket, &["send-keys", "Enter"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_filter_snippets() {
        let snippets = vec![
            Snippet {
                id: "1".into(),
                title: "Docker Shell".into(),
                command: "docker exec -it <id> sh".into(),
                description: "Open container shell".into(),
                tags: vec!["docker".into(), "shell".into()],
            },
            Snippet {
                id: "2".into(),
                title: "Git Log".into(),
                command: "git log --oneline".into(),
                description: "Compact history".into(),
                tags: vec!["git".into()],
            },
        ];
        let matches = filter_snippets(&snippets, "docker");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "1");
    }
}
