use bevy::prelude::*;
use serde::Deserialize;

/// RON wrapper matching the `LoadingTips(...)` format in loading_tips.ron.
#[derive(Debug, Deserialize)]
struct LoadingTips {
    tips: Vec<String>,
}

/// Resource holding all loaded loading tips.
#[derive(Resource, Debug, Clone)]
pub struct LoadingTipsDefs {
    pub tips: Vec<String>,
}

impl Default for LoadingTipsDefs {
    fn default() -> Self {
        Self {
            tips: vec![
                "Tip: To avoid dying, don't get hit.".to_string(),
                "Tip: Enemies deal damage. This is by design.".to_string(),
                "Tip: Items with better stats are generally better than items with worse stats.".to_string(),
            ],
        }
    }
}

impl LoadingTipsDefs {
    /// Get a tip deterministically based on an index (wraps around).
    pub fn get_tip(&self, index: usize) -> &str {
        if self.tips.is_empty() {
            return "Tip: No tips available. This is itself a tip.";
        }
        &self.tips[index % self.tips.len()]
    }
}

/// Load loading tips from `{base}/feel/loading_tips.ron`.
pub fn load_loading_tips(base: &std::path::Path) -> Result<LoadingTipsDefs, String> {
    let path = base.join("feel/loading_tips.ron");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let tips: LoadingTips = ron::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    Ok(LoadingTipsDefs { tips: tips.tips })
}
