use computy_engine::updates::*;

#[test]
fn versions_compare_numerically() {
    assert_eq!(parse_version("v0.10.2"), Some((0, 10, 2)));
    assert_eq!(parse_version("1.2"), None);
    assert_eq!(parse_version("1.2.3-beta"), None);
    assert!(is_newer("v0.10.0", "0.9.9"));
    assert!(is_newer("1.0.0", "0.99.99"));
    assert!(!is_newer("0.4.0", "0.4.0"));
    assert!(!is_newer("0.3.9", "0.4.0"));
    assert!(!is_newer("garbage", "0.4.0"));
}

#[test]
fn installers_match_release_asset_names() {
    assert_eq!(
        installer_name("0.4.0", "linux-x86_64"),
        "Computy_0.4.0_amd64.deb"
    );
    assert_eq!(
        installer_name("0.4.0", "windows-x86_64"),
        "Computy_0.4.0_windows-x86_64.zip"
    );
    assert_eq!(
        installer_name("0.4.0", "macos-aarch64"),
        "Computy_0.4.0_macos-aarch64.dmg"
    );
}

#[test]
fn checksums_are_read_from_the_manifest() {
    let digest = "a".repeat(64);
    let manifest = format!(
        "{}  computy-cli_0.4.0_linux-x86_64.tar.gz\n{digest}  Computy_0.4.0_amd64.deb\n",
        "b".repeat(64)
    );
    assert_eq!(
        checksum_for(&manifest, "Computy_0.4.0_amd64.deb").as_deref(),
        Some(digest.as_str())
    );
    assert_eq!(checksum_for(&manifest, "missing.deb"), None);
    assert_eq!(
        checksum_for("short  Computy_0.4.0_amd64.deb", "Computy_0.4.0_amd64.deb"),
        None
    );
}

#[test]
fn github_release_payloads() {
    let json = r#"{"tag_name":"v0.4.0","draft":false,"prerelease":false,
        "html_url":"https://github.com/ndnci/computy/releases/tag/v0.4.0",
        "published_at":"2026-11-17T18:00:00Z","body":"Notes",
        "assets":[
          {"name":"Computy_0.4.0_amd64.deb","size":10,"browser_download_url":"https://github.com/ndnci/computy/releases/download/v0.4.0/Computy_0.4.0_amd64.deb"},
          {"name":"SHA256SUMS_linux-x86_64.txt","size":1,"browser_download_url":"https://github.com/ndnci/computy/releases/download/v0.4.0/SHA256SUMS_linux-x86_64.txt"}]}"#;
    let (release, manifest) = parse_release(json, "linux-x86_64").unwrap();
    assert_eq!(release.version, "0.4.0");
    assert_eq!(release.notes, "Notes");
    assert_eq!(release.asset.unwrap().size, 10);
    assert!(manifest.unwrap().ends_with("SHA256SUMS_linux-x86_64.txt"));
    // No installer for this platform: the release is still reported.
    let (release, manifest) = parse_release(json, "windows-x86_64").unwrap();
    assert!(release.asset.is_none() && manifest.is_none());
    assert!(
        parse_release(
            &json.replace("\"prerelease\":false", "\"prerelease\":true"),
            "linux-x86_64"
        )
        .is_err()
    );
    assert!(parse_release("{}", "linux-x86_64").is_err());
}

#[test]
fn downloads_only_come_from_the_project_releases() {
    let directory = std::env::temp_dir().join(computy_engine::domain::new_id());
    let asset = |url: &str, sha: &str| Asset {
        name: "Computy_0.4.0_amd64.deb".into(),
        url: url.into(),
        size: 1,
        sha256: sha.into(),
    };
    let sha = "a".repeat(64);
    assert!(
        download(
            &asset("https://example.com/Computy_0.4.0_amd64.deb", &sha),
            &directory
        )
        .is_err()
    );
    assert!(
        download(
            &asset(
                "https://github.com/ndnci/computy/releases/download/v0.4.0/Computy_0.4.0_amd64.deb",
                ""
            ),
            &directory
        )
        .is_err()
    );
    let mut traversal = asset(
        "https://github.com/ndnci/computy/releases/download/v0.4.0/x",
        &sha,
    );
    traversal.name = "../evil.deb".into();
    assert!(download(&traversal, &directory).is_err());
}

#[test]
fn saved_status_reports_availability_against_this_build() {
    let root = std::env::temp_dir().join(computy_engine::domain::new_id());
    std::fs::create_dir_all(&root).unwrap();
    assert!(due(&root, INTERVAL_MS));
    let status = Status {
        checked_at: 1000,
        latest: Some(Release {
            version: "999.0.0".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    std::fs::write(
        root.join("updates.state"),
        serde_json::to_vec(&status).unwrap(),
    )
    .unwrap();
    let loaded = load(&root);
    assert!(loaded.available);
    assert_eq!(loaded.current, CURRENT);
    assert!(!due(&root, 2000));
    assert!(due(&root, 1000 + INTERVAL_MS));
    std::fs::remove_dir_all(root).unwrap();
}
