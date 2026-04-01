use crate::loader::{SkillDefinition, SkillSource};

/// Get all bundled skills
pub fn bundled_skills() -> Vec<SkillDefinition> {
    vec![
        SkillDefinition {
            name: "commit".into(),
            description: "Create a git commit with AI-generated message".into(),
            prompt_template: "Review the staged changes and create a commit with an appropriate message that describes the changes.".into(),
            source: SkillSource::Bundled,
        },
        SkillDefinition {
            name: "review-pr".into(),
            description: "Review a pull request".into(),
            prompt_template: "Review the pull request, checking for bugs, security issues, and code quality.".into(),
            source: SkillSource::Bundled,
        },
        SkillDefinition {
            name: "simplify".into(),
            description: "Review changed code for quality".into(),
            prompt_template: "Review changed code for reuse, quality, simplicity, and potential improvements.".into(),
            source: SkillSource::Bundled,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_skills_count() {
        let skills = bundled_skills();
        assert_eq!(skills.len(), 3);
    }

    #[test]
    fn test_bundled_skills_source() {
        for skill in bundled_skills() {
            assert_eq!(skill.source, SkillSource::Bundled);
        }
    }

    #[test]
    fn test_bundled_skills_names() {
        let skills = bundled_skills();
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"commit"));
        assert!(names.contains(&"review-pr"));
        assert!(names.contains(&"simplify"));
    }

    #[test]
    fn test_bundled_skills_have_descriptions() {
        for skill in bundled_skills() {
            assert!(!skill.description.is_empty());
            assert!(!skill.prompt_template.is_empty());
        }
    }
}
