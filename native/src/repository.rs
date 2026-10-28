use crate::domain::Report;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf};
pub trait ReportRepository: Send + Sync {
    fn get(&self, id: &str) -> Result<serde_json::Value, String>;
    fn delete(&self, id: &str) -> Result<(), String>;
    fn save(&self, report: &Report) -> Result<(), String>;
    fn list(&self) -> Result<ReportList, String>;
    fn rename(&self, id: &str, label: &str) -> Result<(), String>;
}
#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    payload: String,
    sha256: String,
}
#[derive(Debug, Serialize)]
pub struct ReportList {
    pub reports: Vec<Report>,
    pub unreadable: usize,
}
pub struct FileRepository {
    pub root: PathBuf,
}
impl FileRepository {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        Ok(Self { root })
    }
    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || id.len() > 100 || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
            return Err("Identifiant de rapport invalide.".into());
        }
        Ok(self.root.join(format!("{id}.json")))
    }
    pub fn rename(&self, id: &str, label: &str) -> Result<(), String> {
        if label.trim().is_empty() || label.chars().count() > 100 {
            return Err("Nom attendu : 1 à 100 caractères.".into());
        }
        let mut report: Report =
            serde_json::from_value(self.get(id)?).map_err(|e| e.to_string())?;
        report.label = label.trim().into();
        self.save(&report)
    }
}
impl ReportRepository for FileRepository {
    fn rename(&self, id: &str, label: &str) -> Result<(), String> {
        self.rename(id, label)
    }
    fn get(&self, id: &str) -> Result<serde_json::Value, String> {
        let path = self.path(id)?;
        let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 8 * 1024 * 1024
        {
            return Err("Rapport invalide ou trop volumineux.".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let envelope: Envelope =
            serde_json::from_slice(&bytes).map_err(|_| "Rapport illisible.")?;
        if format!("{:x}", Sha256::digest(envelope.payload.as_bytes())) != envelope.sha256 {
            return Err("Empreinte du rapport invalide : fichier altéré.".into());
        }
        let report: Report =
            serde_json::from_str(&envelope.payload).map_err(|_| "Schéma de rapport invalide.")?;
        if report.id != id || report.schema_version != 1 {
            return Err("Version ou identité du rapport invalide.".into());
        }
        serde_json::to_value(report).map_err(|e| e.to_string())
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        std::fs::remove_file(self.path(id)?).map_err(|e| e.to_string())
    }
    fn save(&self, report: &Report) -> Result<(), String> {
        let path = self.path(&report.id)?;
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let envelope = Envelope {
            sha256: format!("{:x}", Sha256::digest(payload.as_bytes())),
            payload,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Rapport invalide ou trop volumineux.".into());
        }
        let temporary = self.root.join(format!("{}.tmp", crate::domain::new_id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        // rename is atomic on Unix. Windows uses ReplaceFile semantics via MoveFileEx below.
        #[cfg(not(windows))]
        let result = std::fs::rename(&temporary, &path);
        #[cfg(windows)]
        let result = {
            use std::os::windows::ffi::OsStrExt;
            let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let success = unsafe {
                windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                    from.as_ptr(),
                    to.as_ptr(),
                    1 | 8,
                )
            };
            if success == 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        };
        if let Err(error) = result {
            let _ = std::fs::remove_file(temporary);
            return Err(error.to_string());
        }
        #[cfg(unix)]
        {
            std::fs::File::open(&self.root)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn list(&self) -> Result<ReportList, String> {
        let mut result = ReportList {
            reports: vec![],
            unreadable: 0,
        };
        for entry in std::fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                match path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| "Nom invalide".to_owned())
                    .and_then(|id| self.get(id))
                    .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))
                {
                    Ok(report) => result.reports.push(report),
                    Err(_) => result.unreadable += 1,
                }
            }
        }
        result
            .reports
            .sort_by_key(|report| std::cmp::Reverse(report.created_at));
        Ok(result)
    }
}
