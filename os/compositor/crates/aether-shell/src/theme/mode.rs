//! Theme mode selection: System, Light, Dark or Water.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
    Water,
}

impl ThemeMode {
    pub const ALL: [Self; 4] = [Self::System, Self::Light, Self::Dark, Self::Water];

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Water => "Water",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Water => "water",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_str().eq_ignore_ascii_case(value))
    }

    /// Resolve `System` to the session default. Aether's session default is
    /// Water, matching the canonical design language.
    pub fn effective(self) -> Self {
        match self {
            Self::System => Self::Water,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_labels_and_resolves_system() {
        for mode in ThemeMode::ALL {
            assert_eq!(ThemeMode::parse(mode.as_str()), Some(mode));
            assert_eq!(
                ThemeMode::parse(&mode.as_str().to_ascii_uppercase()),
                Some(mode)
            );
        }
        assert_eq!(ThemeMode::parse("nope"), None);
        assert_eq!(ThemeMode::System.effective(), ThemeMode::Water);
        assert_eq!(ThemeMode::Light.effective(), ThemeMode::Light);
        assert_eq!(ThemeMode::Water.label(), "Water");
    }
}
