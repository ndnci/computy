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
