use std::path::Path;

#[derive(Debug, Clone)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    pub source: SkillSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Bundled,
    UserDefined,
    Plugin,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid skill format: {message}")]
    InvalidFormat { message: String },
}

/// Load skills from a directory.
///
/// Supports two formats:
/// 1. `<dir>/<name>.md` — single file skill (name from filename, `# Title` as description)
/// 2. `<dir>/<name>/SKILL.md` — directory skill with YAML frontmatter (name, description fields)
pub fn load_skills_from_dir(dir: &Path) -> Result<Vec<SkillDefinition>, SkillError> {
    let mut skills = Vec::new();
    if !dir.exists() {
        return Ok(skills);
    }

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Directory skill: <name>/SKILL.md
            let skill_file = path.join("SKILL.md");
            if skill_file.exists() {
                match load_skill_dir(&path) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        tracing::warn!("Skipping invalid skill dir {}: {}", path.display(), e);
                    }
                }
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            // Single file skill: <name>.md
            match load_skill_file(&path) {
                Ok(skill) => skills.push(skill),
                Err(e) => {
                    tracing::warn!("Skipping invalid skill file {}: {}", path.display(), e);
                }
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Load a skill from a directory containing SKILL.md with YAML frontmatter.
///
/// Expected format:
/// ```text
/// ---
/// name: skill-name
/// description: What this skill does
/// ---
///
/// # Skill Content
/// ...
/// ```
pub fn load_skill_dir(dir: &Path) -> Result<SkillDefinition, SkillError> {
    let skill_file = dir.join("SKILL.md");
    let content = std::fs::read_to_string(&skill_file)?;

    let dir_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    if content.trim().is_empty() {
        return Err(SkillError::InvalidFormat {
            message: "SKILL.md is empty".to_string(),
        });
    }

    // Parse YAML frontmatter
    let (name, description, body) = parse_frontmatter(&content, &dir_name);

    Ok(SkillDefinition {
        name,
        description,
        prompt_template: body,
        source: SkillSource::UserDefined,
    })
}

/// Load a skill from a single .md file.
pub fn load_skill_file(path: &Path) -> Result<SkillDefinition, SkillError> {
    let content = std::fs::read_to_string(path)?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| SkillError::InvalidFormat {
            message: format!("Cannot extract skill name from path: {}", path.display()),
        })?
        .to_string();

    if content.trim().is_empty() {
        return Err(SkillError::InvalidFormat {
            message: "Skill file is empty".to_string(),
        });
    }

    let (parsed_name, description, body) = parse_frontmatter(&content, &name);

    Ok(SkillDefinition {
        name: parsed_name,
        description,
        prompt_template: body,
        source: SkillSource::UserDefined,
    })
}

/// Parse optional YAML frontmatter from a markdown string.
/// Returns (name, description, body_content).
fn parse_frontmatter(content: &str, fallback_name: &str) -> (String, String, String) {
    let trimmed = content.trim();

    if !trimmed.starts_with("---") {
        // No frontmatter — use # heading as description
        let description = trimmed
            .lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").to_string())
            .unwrap_or_else(|| format!("Skill: {}", fallback_name));

        return (fallback_name.to_string(), description, content.to_string());
    }

    // Find closing ---
    let after_first = &trimmed[3..];
    if let Some(end_idx) = after_first.find("\n---") {
        let frontmatter = &after_first[..end_idx];
        let body_start = 3 + end_idx + 4; // skip "---\n---"
        let body = if body_start < trimmed.len() {
            trimmed[body_start..].trim_start().to_string()
        } else {
            String::new()
        };

        // Simple YAML parsing (key: value per line)
        let mut name = fallback_name.to_string();
        let mut description = format!("Skill: {}", fallback_name);

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("name:") {
                name = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("description:") {
                description = val.trim().to_string();
            }
        }

        (name, description, body)
    } else {
        // Malformed frontmatter — treat entire content as body
        (
            fallback_name.to_string(),
            format!("Skill: {}", fallback_name),
            content.to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_skill_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-skill.md");
        fs::write(&path, "# My Test Skill\n\nDo something useful.").unwrap();

        let skill = load_skill_file(&path).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "My Test Skill");
        assert!(skill.prompt_template.contains("Do something useful"));
        assert_eq!(skill.source, SkillSource::UserDefined);
    }

    #[test]
    fn test_load_skill_file_with_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("my-skill.md");
        fs::write(
            &path,
            "---\nname: custom-name\ndescription: A custom skill\n---\n\n# Content\nHello",
        )
        .unwrap();

        let skill = load_skill_file(&path).unwrap();
        assert_eq!(skill.name, "custom-name");
        assert_eq!(skill.description, "A custom skill");
        assert!(skill.prompt_template.contains("# Content"));
    }

    #[test]
    fn test_load_skill_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Does things\n---\n\nPrompt here.",
        )
        .unwrap();

        let skill = load_skill_dir(&skill_dir).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.description, "Does things");
        assert!(skill.prompt_template.contains("Prompt here"));
    }

    #[test]
    fn test_load_skill_file_no_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.md");
        fs::write(&path, "Just some prompt text.").unwrap();

        let skill = load_skill_file(&path).unwrap();
        assert_eq!(skill.name, "plain");
        assert_eq!(skill.description, "Skill: plain");
    }

    #[test]
    fn test_load_skill_file_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.md");
        fs::write(&path, "   ").unwrap();

        let result = load_skill_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_skills_from_dir_mixed() {
        let dir = tempfile::tempdir().unwrap();

        // File-based skill
        fs::write(dir.path().join("alpha.md"), "# Alpha\nAlpha prompt.").unwrap();

        // Directory-based skill
        let skill_dir = dir.path().join("beta");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: beta\ndescription: Beta skill\n---\n\nBeta prompt.",
        )
        .unwrap();

        // Not a skill
        fs::write(dir.path().join("not-a-skill.txt"), "ignored").unwrap();

        // Empty dir without SKILL.md
        fs::create_dir(dir.path().join("empty-dir")).unwrap();

        let skills = load_skills_from_dir(dir.path()).unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "alpha");
        assert_eq!(skills[1].name, "beta");
    }

    #[test]
    fn test_load_skills_from_nonexistent_dir() {
        let skills = load_skills_from_dir(Path::new("/nonexistent/path")).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_skill_source_eq() {
        assert_eq!(SkillSource::Bundled, SkillSource::Bundled);
        assert_ne!(SkillSource::Bundled, SkillSource::UserDefined);
        assert_ne!(SkillSource::UserDefined, SkillSource::Plugin);
    }

    #[test]
    fn test_parse_frontmatter_none() {
        let (name, desc, body) = parse_frontmatter("# Hello\nWorld", "fallback");
        assert_eq!(name, "fallback");
        assert_eq!(desc, "Hello");
        assert!(body.contains("World"));
    }

    #[test]
    fn test_parse_frontmatter_with_yaml() {
        let content = "---\nname: test\ndescription: A test\n---\n\nBody";
        let (name, desc, body) = parse_frontmatter(content, "fallback");
        assert_eq!(name, "test");
        assert_eq!(desc, "A test");
        assert_eq!(body, "Body");
    }

    #[test]
    fn test_parse_frontmatter_malformed() {
        let content = "---\nname: test\nno closing marker";
        let (name, _, _) = parse_frontmatter(content, "fallback");
        assert_eq!(name, "fallback"); // falls back because no closing ---
    }
}
