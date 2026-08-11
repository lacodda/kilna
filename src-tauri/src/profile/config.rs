use serde::{Deserialize, Serialize};

/// A craft scenario. Everything that differs between music, prose and podcasting
/// is described here rather than in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Kinds a work can take: song, chapter, episode.
    pub work_kinds: Vec<Kind>,
    /// Kinds a release can take: clip, short, audio release.
    pub release_kinds: Vec<Kind>,
    /// Kinds a collection can take: album, book, season.
    pub collection_kinds: Vec<Kind>,
    /// Independent bodies a work carries: lyrics and style, or text and outline.
    pub version_roles: Vec<Kind>,
    /// Statuses a work moves through, in order.
    pub statuses: Vec<Kind>,
    /// What a work is judged on.
    pub axes: Vec<Axis>,
    /// Score thresholds, highest `min` first when evaluated.
    pub tiers: Vec<Tier>,
    /// Craft-specific fields stored in `work.meta`.
    pub work_meta_fields: Vec<MetaField>,
}

/// A vocabulary entry: a stable key with a label the user may rename.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kind {
    pub key: String,
    pub label: String,
}

impl Kind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
        }
    }
}

/// One dimension a work is scored along.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axis {
    pub key: String,
    pub label: String,
    /// Relative importance when the axes are combined into a total.
    pub weight: f64,
    /// Highest value the axis accepts; scores are normalised against it.
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A band a total score falls into. `min` is on the normalised 0–100 total.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier {
    pub key: String,
    pub label: String,
    pub min: f64,
}

/// A typed field inside `work.meta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: MetaFieldType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MetaFieldType {
    Text,
    Number,
    Date,
    Boolean,
}

impl ProfileConfig {
    /// Combine axis values into a 0–100 total.
    ///
    /// Axes missing from `values` are skipped rather than counted as zero: a
    /// half-filled score card should not read as a bad work.
    pub fn total(&self, values: &serde_json::Map<String, serde_json::Value>) -> f64 {
        let mut weighted = 0.0;
        let mut weight_sum = 0.0;

        for axis in &self.axes {
            let Some(value) = values.get(&axis.key).and_then(serde_json::Value::as_f64) else {
                continue;
            };
            if axis.scale <= 0.0 {
                continue;
            }
            weighted += (value / axis.scale) * axis.weight;
            weight_sum += axis.weight;
        }

        if weight_sum == 0.0 {
            return 0.0;
        }
        (weighted / weight_sum) * 100.0
    }

    /// The highest tier whose threshold the total reaches.
    pub fn tier_for(&self, total: f64) -> Option<&Tier> {
        self.tiers
            .iter()
            .filter(|tier| total >= tier.min)
            .max_by(|a, b| a.min.total_cmp(&b.min))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> ProfileConfig {
        serde_json::from_value(json!({
            "work_kinds": [{ "key": "song", "label": "Song" }],
            "release_kinds": [{ "key": "clip", "label": "Clip" }],
            "collection_kinds": [{ "key": "album", "label": "Album" }],
            "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
            "statuses": [{ "key": "draft", "label": "Draft" }],
            "axes": [
                { "key": "hook", "label": "Hook", "weight": 2.0, "scale": 10.0 },
                { "key": "text", "label": "Text", "weight": 1.0, "scale": 10.0 }
            ],
            "tiers": [
                { "key": "hold", "label": "Hold", "min": 0.0 },
                { "key": "clip", "label": "Clip", "min": 75.0 }
            ],
            "work_meta_fields": []
        }))
        .unwrap()
    }

    #[test]
    fn total_weighs_the_axes() {
        let values = json!({ "hook": 10.0, "text": 4.0 });
        // (1.0 * 2 + 0.4 * 1) / 3 = 0.8
        let total = config().total(values.as_object().unwrap());
        assert!((total - 80.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn a_missing_axis_does_not_count_as_zero() {
        let values = json!({ "hook": 8.0 });
        let total = config().total(values.as_object().unwrap());
        assert!((total - 80.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn an_empty_score_card_totals_zero_rather_than_dividing_by_zero() {
        let values = json!({});
        assert_eq!(config().total(values.as_object().unwrap()), 0.0);
    }

    #[test]
    fn tier_picks_the_highest_threshold_reached() {
        let config = config();
        assert_eq!(config.tier_for(80.0).unwrap().key, "clip");
        assert_eq!(config.tier_for(74.9).unwrap().key, "hold");
    }
}
