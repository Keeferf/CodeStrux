use std::path::Path;

/// Validates that `name` is a safe, single-component `.gguf` filename.
///
/// # Errors
///
/// Returns an error if the filename contains path separators, control
/// characters, or does not end with `.gguf`.
pub fn sanitise_filename(name: &str) -> Result<(), String> {
    let p = Path::new(name);
    if p.components().count() != 1 {
        return Err(format!("Invalid filename: '{}'", name));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(format!("Invalid filename: '{}'", name));
    }
    if !name.ends_with(".gguf") {
        return Err("Only .gguf files are supported".to_string());
    }
    Ok(())
}

/// Validates a `owner/repo` model ID and returns it with `/` replaced by `__`.
///
/// Requires exactly one `/` separator. Neither part may be empty, equal `..`,
/// or contain `\` or `\0`.
///
/// # Errors
///
/// Returns an error if the model ID has the wrong shape or contains illegal
/// characters.
pub fn sanitise_model_id(id: &str) -> Result<String, String> {
    let slash_count = id.chars().filter(|&c| c == '/').count();
    if slash_count != 1 {
        return Err(format!("Invalid model ID (expected 'owner/repo'): '{}'", id));
    }

    // Safe to unwrap: exactly one '/' was confirmed above.
    let (owner, repo) = id.split_once('/').unwrap();

    for (label, part) in [("owner", owner), ("repo", repo)] {
        if part.is_empty() {
            return Err(format!("Invalid model ID ({} is empty): '{}'", label, id));
        }
        // After `split_once('/')`, neither part can contain '/', so the only
        // traversal value possible is the literal string "..".
        if part == ".." {
            return Err(format!("Invalid model ID ({} traversal): '{}'", label, id));
        }
        if part.contains('\\') || part.contains('\0') {
            return Err(format!("Invalid model ID ({} contains illegal character): '{}'", label, id));
        }
    }

    Ok(id.replace('/', "__"))
}