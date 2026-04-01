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

/// Load skills from a directory. Each `.md` file in the directory is treated as a skill.
/// The file name (without extension) becomes the skill name.
/// The first line starting with `#` is used as the description.
/// The rest of the file is the prompt template.
pub fn load_skills_from_dir(dir: &Path) -> Result<Vec<SkillDefinition>, SkillError> {
    let mut skills = Vec::new();
    if !dir.exists() {
        return Ok(skills);
    }

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
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

/// Load a skill from a single file.
/// The file name (without extension) is the skill name.
/// The first line starting with `# ` is used as the description.
/// The entire file content is used as the prompt template.
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

    let description = content
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").to_string())
        .unwrap_or_else(|| format!("Skill: {name}"));

    Ok(SkillDefinition {
        name,
        description,
        prompt_template: content,
        source: SkillSource::UserDefined,
    })
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
    fn test_load_skills_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("alpha.md"), "# Alpha\nAlpha prompt.").unwrap();
        fs::write(dir.path().join("beta.md"), "# Beta\nBeta prompt.").unwrap();
        fs::write(dir.path().join("not-a-skill.txt"), "ignored").unwrap();

        let skills = load_skills_from_dir(dir.path()).unwrap();
        assert_eq!(skills.len(), 2);
        // sorted by name
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
}
