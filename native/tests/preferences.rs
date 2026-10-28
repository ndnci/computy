use computy_engine::preferences::{Preferences, load, save};
#[test]
fn follows_system_by_default_and_supports_locale_preferences() {
    let defaults = Preferences::default();
    assert_eq!(defaults.theme, "system");
    let value = serde_json::json!({"theme":"system","language":"ar"});
    let preferences = serde_json::from_value::<Preferences>(value);
    assert!(preferences.is_ok());
    let root = std::env::temp_dir().join(format!(
        "computy-prefs-{}",
        computy_engine::domain::new_id()
    ));
    save(&root, &preferences.unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(load(&root).unwrap()).unwrap()["language"],
        "ar"
    );
    std::fs::remove_dir_all(root).unwrap();
}
