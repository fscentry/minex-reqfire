use tracing::info;

pub fn load_bodies_from_file(
    path: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed reading body_file '{}': {}", path, e))?;

    let mut bodies: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(json) => bodies.push(json),
            Err(e) => {
                errors.push(format!(
                    "  Line {}: {} → \"{}\"",
                    line_num + 1,
                    e,
                    if trimmed.len() > 60 { &trimmed[..60] } else { trimmed }
                ));
            }
        }
    }
    
    if !errors.is_empty() {
        return Err(format!(
            "found {} invalid line from JSON  '{}':\n{}",
            errors.len(),
            path,
            errors.join("\n")
        ).into());
    }

    if bodies.is_empty() {
        return Err(format!(
            "File '{}' doest not have valid JSON line (empty file?)",
            path
        ).into());
    }

    info!(
        "📄 Body file '{}' loaded: {} request bodies",
        path,
        bodies.len()
    );

    Ok(bodies)
}