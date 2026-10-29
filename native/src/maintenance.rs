//! Explicit removal of Computy-owned data only. Never exposed through MCP.
use std::{fs, path::Path};

pub fn erase_data(root: &Path, confirmed: bool) -> Result<(), String> {
    if !confirmed {
        return Err("Explicit confirmation is required.".into());
    }
    let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("The data directory must be a real directory.".into());
    }
    let names = [
        "reports",
        "scratch",
        "preferences.state",
        "boot-diagnostic.state",
        "diagnostic.lock",
    ];
    for name in names {
        match fs::symlink_metadata(root.join(name)) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("A data path is a symbolic link.".into());
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(error.to_string());
            }
            _ => {}
        }
    }
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("diagnostic.lock"))
        .map_err(|e| e.to_string())?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .map_err(|_| "Another Computy process is using these data.".to_owned())?;
    for name in [
        "reports",
        "scratch",
        "preferences.state",
        "boot-diagnostic.state",
    ] {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if meta.is_dir() {
                    fs::remove_dir_all(path)
                } else {
                    fs::remove_file(path)
                }
                .map_err(|e| e.to_string())?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    // Keep the empty lock inode: deleting it would break exclusion for waiters.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("computy-erase-{}", crate::domain::new_id()));
        fs::create_dir_all(root.join("reports")).unwrap();
        fs::write(root.join("reports/1.json"), "private report").unwrap();
        fs::write(root.join("preferences.state"), "settings").unwrap();
        fs::write(root.join("export.json"), "user export").unwrap();
        root
    }
    #[test]
    fn erasure_requires_consent_and_preserves_unmanaged_files() {
        let root = fixture();
        assert!(erase_data(&root, false).is_err());
        assert!(root.join("reports/1.json").exists());
        erase_data(&root, true).unwrap();
        assert!(!root.join("reports").exists());
        assert!(!root.join("preferences.state").exists());
        assert_eq!(
            fs::read_to_string(root.join("export.json")).unwrap(),
            "user export"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn erasure_refuses_an_active_diagnostic_lock() {
        let root = fixture();
        let lock = fs::File::create(root.join("diagnostic.lock")).unwrap();
        fs2::FileExt::lock_exclusive(&lock).unwrap();
        assert!(erase_data(&root, true).is_err());
        assert!(root.join("reports/1.json").exists());
        drop(lock);
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn erasure_refuses_symlinked_data_before_deleting_anything() {
        let root = fixture();
        std::os::unix::fs::symlink(root.join("export.json"), root.join("boot-diagnostic.state"))
            .unwrap();
        assert!(erase_data(&root, true).is_err());
        assert!(root.join("reports/1.json").exists());
        assert_eq!(
            fs::read_to_string(root.join("export.json")).unwrap(),
            "user export"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
