use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Metadata {
    DisplayName,
    Pronouns,
}

pub fn format(
    nickname: &str,
    enabled: &[Metadata],
    display_name: Option<&str>,
    pronouns: Option<&str>,
) -> String {
    let display_name = enabled
        .contains(&Metadata::DisplayName)
        .then_some(display_name)
        .flatten()
        .filter(|s| !s.is_empty());
    let pronouns = enabled
        .contains(&Metadata::Pronouns)
        .then_some(pronouns)
        .flatten()
        .filter(|s| !s.is_empty());

    match (display_name, pronouns) {
        (Some(display_name), Some(pronouns)) => {
            format!("{display_name} ({nickname}, {pronouns})")
        }
        (Some(display_name), None) => format!("{display_name} ({nickname})"),
        (None, Some(pronouns)) => format!("{nickname} ({pronouns})"),
        (None, None) => nickname.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Metadata, format};

    #[test]
    fn keeps_plain_nickname_when_no_metadata_enabled() {
        assert_eq!(
            format("storm", &[], Some("Casper"), Some("he/him")),
            "storm"
        );
    }

    #[test]
    fn formats_display_name_when_present() {
        assert_eq!(
            format("storm", &[Metadata::DisplayName], Some("Casper"), None),
            "Casper (storm)"
        );
    }

    #[test]
    fn skips_missing_display_name() {
        assert_eq!(
            format("storm", &[Metadata::DisplayName], None, None),
            "storm"
        );
    }

    #[test]
    fn formats_pronouns_when_present() {
        assert_eq!(
            format("storm", &[Metadata::Pronouns], None, Some("he/him")),
            "storm (he/him)"
        );
    }

    #[test]
    fn combines_display_name_and_pronouns() {
        assert_eq!(
            format(
                "storm",
                &[Metadata::Pronouns, Metadata::DisplayName],
                Some("Casper"),
                Some("he/him"),
            ),
            "Casper (storm, he/him)"
        );
    }
}
