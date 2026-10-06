//! Parameter extraction and substitution for command templates.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParam {
    pub name: String,
    pub default: Option<String>,
}

/// Extracts parameters enclosed in `<...>` from the command template.
/// Supports `<name>` and `<name:default_value>`.
pub fn extract_parameters(template: &str) -> Vec<TemplateParam> {
    let mut params = Vec::new();
    let mut chars = template.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch == '<' {
            let mut end_idx = None;
            let mut inner = String::new();
            for (idx, c) in chars.by_ref() {
                if c == '>' {
                    end_idx = Some(idx);
                    break;
                }
                inner.push(c);
            }
            if end_idx.is_some() && !inner.is_empty() {
                let (name, default) = match inner.split_once(':') {
                    Some((n, d)) => (n.trim().to_string(), Some(d.to_string())),
                    None => (inner.trim().to_string(), None),
                };
                if !name.is_empty() && !params.iter().any(|p: &TemplateParam| p.name == name) {
                    params.push(TemplateParam { name, default });
                }
            } else if end_idx.is_none() {
                break;
            }
        }
    }
    params
}

/// Substitutes parameters in the command template using the provided (name, value) pairs.
pub fn substitute_parameters(template: &str, values: &[(String, String)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch == '<' {
            let mut end_idx = None;
            let mut inner = String::new();
            for (idx, c) in chars.by_ref() {
                if c == '>' {
                    end_idx = Some(idx);
                    break;
                }
                inner.push(c);
            }
            if end_idx.is_some() && !inner.is_empty() {
                let (name, default) = match inner.split_once(':') {
                    Some((n, d)) => (n.trim(), Some(d)),
                    None => (inner.trim(), None),
                };
                if let Some((_, val)) = values.iter().find(|(k, _)| k == name) {
                    if val.is_empty() {
                        if let Some(def) = default {
                            result.push_str(def);
                        }
                    } else {
                        result.push_str(val);
                    }
                } else if let Some(def) = default {
                    result.push_str(def);
                } else {
                    result.push('<');
                    result.push_str(&inner);
                    result.push('>');
                }
            } else {
                result.push('<');
                result.push_str(&inner);
                if end_idx.is_some() {
                    result.push('>');
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_and_substitute_parameters() {
        let cmd = "docker exec -it <container_id> <shell:bash>";
        let params = extract_parameters(cmd);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "container_id");
        assert_eq!(params[0].default, None);
        assert_eq!(params[1].name, "shell");
        assert_eq!(params[1].default, Some("bash".into()));

        let values = [
            ("container_id".into(), "my-app".into()),
            ("shell".into(), "sh".into()),
        ];
        let result = substitute_parameters(cmd, &values);
        assert_eq!(result, "docker exec -it my-app sh");
    }

    #[test]
    fn test_substitute_with_default_fallback() {
        let cmd = "git checkout -b <branch:feature>";
        let values = [("branch".into(), "".into())];
        let result = substitute_parameters(cmd, &values);
        assert_eq!(result, "git checkout -b feature");
    }
}
