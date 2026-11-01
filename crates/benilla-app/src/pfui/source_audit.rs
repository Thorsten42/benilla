//! Source audits must cover the same optional modules as the tested client.

pub(super) fn is_compiled(path: &std::path::Path) -> bool {
    let rel = super::rel_path(path);
    let name = rel.rsplit('/').next().unwrap_or_default();
    cfg!(feature = "pfui")
        || !(rel.starts_with("pfui/") || matches!(name, "pfui.rs" | "pfui_tests.rs"))
}

#[test]
fn optional_sources_follow_the_build_feature() {
    for path in [
        "pfui.rs",
        "net/pfui.rs",
        "ui_social/pfui.rs",
        "ui_aura/pfui_tests.rs",
    ] {
        assert_eq!(
            is_compiled(&super::src_dir().join(path)),
            cfg!(feature = "pfui")
        );
    }
    assert!(is_compiled(&super::src_dir().join("spell/cooldowns.rs")));
}
