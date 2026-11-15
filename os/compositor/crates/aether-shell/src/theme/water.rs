//! Water tone, motion, intensity and background parameter enums.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterTone {
    Lagoon,
    Ocean,
    Deep,
}

impl WaterTone {
    pub const ALL: [Self; 3] = [Self::Lagoon, Self::Ocean, Self::Deep];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lagoon => "Lagoon",
            Self::Ocean => "Ocean",
            Self::Deep => "Deep",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lagoon => "lagoon",
            Self::Ocean => "ocean",
            Self::Deep => "deep",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|tone| tone.as_str().eq_ignore_ascii_case(value))
    }

    /// Accent RGB. Light mode uses the deeper variants from `tokens.css`;
    /// dark and Water use the bright Water tones.
    pub fn rgb(self, light: bool) -> [u8; 3] {
        match (self, light) {
            (Self::Lagoon, false) => [85, 240, 224],
            (Self::Ocean, false) => [56, 214, 255],
            (Self::Deep, false) => [72, 183, 255],
            (Self::Lagoon, true) => [10, 143, 127],
            (Self::Ocean, true) => [6, 122, 153],
            (Self::Deep, true) => [33, 102, 196],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterSpeed {
    Calm,
    Flowing,
    Rapid,
}

impl WaterSpeed {
    pub const ALL: [Self; 3] = [Self::Calm, Self::Flowing, Self::Rapid];

    pub fn label(self) -> &'static str {
        match self {
            Self::Calm => "Calm",
            Self::Flowing => "Flowing",
            Self::Rapid => "Rapid",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Flowing => "flowing",
            Self::Rapid => "rapid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|speed| speed.as_str().eq_ignore_ascii_case(value))
    }

    pub fn factor(self) -> f32 {
        match self {
            Self::Calm => 0.65,
            Self::Flowing => 1.0,
            Self::Rapid => 1.6,
        }
    }

    /// Visible wave bands in the themed backdrop and the Themes preview.
    pub fn wave_count(self) -> usize {
        match self {
            Self::Calm => 2,
            Self::Flowing => 3,
            Self::Rapid => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterIntensity {
    Subtle,
    Balanced,
    Lively,
}

impl WaterIntensity {
    pub const ALL: [Self; 3] = [Self::Subtle, Self::Balanced, Self::Lively];

    pub fn label(self) -> &'static str {
        match self {
            Self::Subtle => "Subtle",
            Self::Balanced => "Balanced",
            Self::Lively => "Lively",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Subtle => "subtle",
            Self::Balanced => "balanced",
            Self::Lively => "lively",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|intensity| intensity.as_str().eq_ignore_ascii_case(value))
    }

    /// Ring size/alpha step used by the preview.
    pub fn level(self) -> u32 {
        match self {
            Self::Subtle => 0,
            Self::Balanced => 1,
            Self::Lively => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterBackground {
    Still,
    Sea,
}

impl WaterBackground {
    pub const ALL: [Self; 2] = [Self::Still, Self::Sea];

    pub fn label(self) -> &'static str {
        match self {
            Self::Still => "Still gradient",
            Self::Sea => "Sea and waves",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Still => "still",
            Self::Sea => "sea",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|background| background.as_str().eq_ignore_ascii_case(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_value() {
        for tone in WaterTone::ALL {
            assert_eq!(WaterTone::parse(tone.as_str()), Some(tone));
        }
        for speed in WaterSpeed::ALL {
            assert_eq!(WaterSpeed::parse(speed.as_str()), Some(speed));
        }
        for intensity in WaterIntensity::ALL {
            assert_eq!(WaterIntensity::parse(intensity.as_str()), Some(intensity));
        }
        for background in WaterBackground::ALL {
            assert_eq!(
                WaterBackground::parse(background.as_str()),
                Some(background)
            );
        }
        assert_eq!(WaterTone::parse("nope"), None);
    }

    #[test]
    fn accent_and_motion_scales_match_the_reference() {
        assert_eq!(WaterTone::Ocean.rgb(false), [56, 214, 255]);
        assert_eq!(WaterTone::Ocean.rgb(true), [6, 122, 153]);
        assert_eq!(WaterSpeed::Rapid.factor(), 1.6);
        assert_eq!(WaterSpeed::Calm.wave_count(), 2);
        assert_eq!(WaterSpeed::Rapid.wave_count(), 4);
        assert_eq!(WaterIntensity::Lively.level(), 2);
        assert_eq!(WaterBackground::Sea.label(), "Sea and waves");
    }
}
