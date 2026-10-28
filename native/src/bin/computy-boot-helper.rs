// Installed root-owned, invoked only through the dedicated PolicyKit action.
fn main() {
    #[cfg(target_os = "linux")]
    let result = {
        let args: Vec<_> = std::env::args().collect();
        let uid = std::env::var("PKEXEC_UID")
            .ok()
            .and_then(|uid| uid.parse::<u32>().ok());
        match (args.as_slice(), uid) {
            ([_, action], Some(uid)) => computy_engine::linux_boot::privileged::run(action, uid),
            _ => Err("boot:authorization_required".into()),
        }
    };
    #[cfg(not(target_os = "linux"))]
    let result: Result<serde_json::Value, String> = Err("boot:unsupported_platform".into());
    match result {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!("{}", serde_json::json!({"error":error}));
            std::process::exit(1);
        }
    }
}
