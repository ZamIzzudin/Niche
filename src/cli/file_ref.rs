use std::path::Path;

pub fn expand_file_refs(input: &str) -> String {
    let mut refs: Vec<(String, String)> = Vec::new();

    for word in input.split_whitespace() {
        if let Some(path_str) = word.strip_prefix('@') {
            if path_str.is_empty() {
                continue;
            }
            let path = Path::new(path_str);
            if path.is_file() {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        refs.push((path_str.to_string(), content));
                    }
                    Err(e) => {
                        eprintln!("Warning: could not read @{path_str}: {e}");
                    }
                }
            }
        }
    }

    if refs.is_empty() {
        return input.to_string();
    }

    let mut result = input.to_string();
    result.push_str("\n\n");
    for (path, content) in &refs {
        result.push_str(&format!("--- {path} ---\n{content}\n--- End of {path} ---\n"));
    }
    result
}
