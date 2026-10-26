use crate::settings::Language;
use std::{collections::BTreeMap, sync::LazyLock};

static EN: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../locales/en.json")).expect("valid English catalog")
});
static ZH: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../locales/zh-CN.json")).expect("valid Chinese catalog")
});

pub fn tr(language: Language, key: &str) -> &str {
    let catalog = match language {
        Language::English => &*EN,
        Language::Chinese => &*ZH,
    };
    catalog
        .get(key)
        .or_else(|| EN.get(key))
        .map(String::as_str)
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogs_have_matching_nonempty_keys() {
        assert_eq!(EN.keys().collect::<Vec<_>>(), ZH.keys().collect::<Vec<_>>());
        assert!(EN.values().chain(ZH.values()).all(|s| !s.trim().is_empty()));
        assert_eq!(tr(Language::Chinese, "record"), "开始录制");
    }
}
