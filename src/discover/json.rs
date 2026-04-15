use std::collections::HashMap;

/// Simple JSON object extractor — pulls key-value pairs from a top-level object field.
/// Not a full JSON parser but handles the common case of `"scripts": { "key": "value" }`.
pub fn extract_json_object(json: &str, field: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let search = format!("\"{field}\"");
    let field_start = match json.find(&search) {
        Some(pos) => pos + search.len(),
        None => return result,
    };

    let rest = &json[field_start..];
    let brace_start = match rest.find('{') {
        Some(pos) => field_start + pos,
        None => return result,
    };

    // Find matching closing brace
    let mut depth = 0;
    let mut brace_end = brace_start;
    for (i, c) in json[brace_start..].chars().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    brace_end = brace_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let obj = &json[brace_start + 1..brace_end];

    // Extract "key": "value" pairs
    let mut chars = obj.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '"' {
            chars.next();
            let key: String = chars.by_ref().take_while(|&c| c != '"').collect();
            // Skip to colon
            while let Some(&c) = chars.peek() {
                if c == ':' {
                    chars.next();
                    break;
                }
                chars.next();
            }
            // Skip whitespace
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            // Read value
            if let Some(&'"') = chars.peek() {
                chars.next();
                let value: String = chars.by_ref().take_while(|&c| c != '"').collect();
                result.insert(key, value);
            }
        } else {
            chars.next();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_scripts() {
        let json = r#"{
            "name": "myapp",
            "scripts": {
                "dev": "vite",
                "build": "vite build",
                "test": "vitest"
            }
        }"#;
        let scripts = extract_json_object(json, "scripts");
        assert_eq!(scripts.get("dev").unwrap(), "vite");
        assert_eq!(scripts.get("build").unwrap(), "vite build");
        assert_eq!(scripts.get("test").unwrap(), "vitest");
    }
}
