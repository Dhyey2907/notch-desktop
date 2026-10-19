//! Panel layouts: per panel, the order of its sections, which are hidden, and how big the
//! panel opens; plus one spacing choice for every panel. The panels own their section ids;
//! here they're only names, so a section that no longer exists is simply ignored.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

const MAX_PANELS: usize = 128;
const MAX_IDS: usize = 64;
const MAX_ID_LEN: usize = 64;

/// How much room between things, everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

/// How big a panel opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PanelSize {
    #[default]
    Normal,
    Tall,
    Wide,
    Large,
}

/// One panel's (or one list's, like Control's toggles) arrangement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PanelLayout {
    /// Section ids in the order chosen; ones not listed keep their built-in place after these.
    pub order: Vec<String>,
    pub hidden: Vec<String>,
    pub size: PanelSize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct LayoutSettings {
    pub density: Density,
    /// By panel id ("control", "home"…) or list id ("control.toggles").
    pub panels: BTreeMap<String, PanelLayout>,
}

fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl LayoutSettings {
    pub fn valid(&self) -> bool {
        self.panels.len() <= MAX_PANELS
            && self.panels.iter().all(|(id, p)| {
                id_ok(id)
                    && p.order.len() <= MAX_IDS
                    && p.hidden.len() <= MAX_IDS
                    && p.order.iter().chain(&p.hidden).all(|s| id_ok(s))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_ids_only() {
        let mut s = LayoutSettings::default();
        assert!(s.valid());
        s.panels.insert(
            "control.toggles".into(),
            PanelLayout {
                order: vec!["wifi".into(), "night-light".into()],
                hidden: vec!["hotspot".into()],
                size: PanelSize::Tall,
            },
        );
        assert!(s.valid());
        s.panels.insert("bad id!".into(), PanelLayout::default());
        assert!(!s.valid());
    }
}
