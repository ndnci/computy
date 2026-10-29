use computy_engine::{
    preferences::Preferences,
    startup::{LaunchMode, launch_mode},
};
fn args(flag: &str) -> Vec<String> {
    vec!["Computy".into(), flag.into()]
}
#[test]
fn normal_launch_opens_the_window_but_login_starts_only_the_monitor() {
    let prefs = Preferences::default();
    assert_eq!(launch_mode(&args(""), &prefs), LaunchMode::Window);
    assert_eq!(
        launch_mode(&args("--background"), &prefs),
        LaunchMode::Monitor
    );
    assert_eq!(
        launch_mode(&args("--return-from-diagnostic"), &prefs),
        LaunchMode::Monitor
    );
}
#[test]
fn background_start_without_enabled_indicators_exits_without_a_window() {
    let mut prefs = Preferences {
        tray_enabled: false,
        ..Preferences::default()
    };
    assert_eq!(launch_mode(&args("--background"), &prefs), LaunchMode::Exit);
    prefs.tray_enabled = true;
    prefs.tray_components.clear();
    assert_eq!(launch_mode(&args("--background"), &prefs), LaunchMode::Exit);
    prefs.tray_disks.push("/".into());
    assert_eq!(
        launch_mode(&args("--background"), &prefs),
        LaunchMode::Monitor
    );
}
#[test]
fn login_defaults_on_and_keeps_explicit_opt_out() {
    let root = std::env::temp_dir().join(computy_engine::domain::new_id());
    std::fs::create_dir_all(&root).unwrap();
    assert!(computy_engine::startup::desired(&root).unwrap());
    computy_engine::startup::save_desired(&root, false).unwrap();
    assert!(!computy_engine::startup::desired(&root).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
