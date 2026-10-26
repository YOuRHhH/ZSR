use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    #[default]
    Original,
    P480,
    P720,
    P1080,
    P1440,
    P2160,
}

impl Resolution {
    pub const ALL: [Self; 6] = [
        Self::Original,
        Self::P480,
        Self::P720,
        Self::P1080,
        Self::P1440,
        Self::P2160,
    ];

    pub fn dimensions(self) -> Option<(u32, u32)> {
        match self {
            Self::Original => None,
            Self::P480 => Some((854, 480)),
            Self::P720 => Some((1280, 720)),
            Self::P1080 => Some((1920, 1080)),
            Self::P1440 => Some((2560, 1440)),
            Self::P2160 => Some((3840, 2160)),
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Original => "resolution_original",
            Self::P480 => "resolution_480",
            Self::P720 => "resolution_720",
            Self::P1080 => "resolution_1080",
            Self::P1440 => "resolution_1440",
            Self::P2160 => "resolution_2160",
        }
    }

    /// Preserve the whole source; pad nonmatching aspect ratios instead of stretching.
    pub fn video_filter(self) -> String {
        match self.dimensions() {
            None => "scale=trunc(iw/2)*2:trunc(ih/2)*2,setsar=1".into(),
            Some((width, height)) => format!(
                "scale={width}:{height}:force_original_aspect_ratio=decrease:force_divisible_by=2,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    #[test]
    fn old_preferences_keep_original_resolution_and_presets_round_trip() {
        let legacy: Settings = serde_json::from_str(r#"{"fps":60,"crf":18}"#).unwrap();
        assert_eq!(legacy.resolution, Resolution::Original);
        for resolution in Resolution::ALL {
            let settings = Settings {
                resolution,
                ..Settings::default()
            };
            let restored: Settings =
                serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
            assert_eq!(restored.resolution, resolution);
        }
    }
}
