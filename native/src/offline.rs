use crate::{
    domain::{Check, Status},
    service::AppService,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
pub fn parse_windows_event(event: &Value, since: u64) -> Option<Check> {
    if event["provider"].as_str()? != "Microsoft-Windows-MemoryDiagnostics-Results"
        || event["created_at"].as_u64()? < since
    {
        return None;
    }
    let id = event["event_id"].as_u64()?;
    if ![1101, 1102, 1201, 1202].contains(&id) {
        return None;
    }
    let bad_pages = event["num_bad_pages"]
        .as_u64()
        .or_else(|| event["num_bad_pages"].as_str().and_then(|v| v.parse().ok()))
        .unwrap_or(0);
    let completion = event["completion_type"].as_str().unwrap_or("");
    let status =
        if [1102, 1202].contains(&id) || bad_pages > 0 || completion.eq_ignore_ascii_case("fail") {
            Status::Failed
        } else if !completion.is_empty()
            && !completion.eq_ignore_ascii_case("pass")
            && !completion.eq_ignore_ascii_case("success")
        {
            Status::Unavailable
        } else {
            Status::Passed
        };
    let mut check = Check::new(
        "windows-boot-memory",
        status,
        match status {
            Status::Passed => "Windows ne signale aucune erreur dans ce diagnostic au démarrage.",
            Status::Failed => "Le diagnostic mémoire de Windows signale des erreurs matérielles.",
            _ => "Le résultat Windows indique un diagnostic incomplet.",
        },
    );
    check.component = "memory".into();
    check.name = "Diagnostic mémoire Windows · au démarrage".into();
    check.metrics = event.clone();
    check.limitations=vec!["Résultat importé du journal système Windows, distinct du moteur de motifs Computy.".into(),"L’événement doit être postérieur à la demande. Sa couverture physique n’est pas déduite du seul code de réussite.".into(),"Le matériel est inventorié au retour dans l’OS ; il n’est pas mesuré par Computy pendant le test autonome.".into()];
    Some(check)
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Pending {
    requested_at: u64,
    machine_id: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
    pending: Option<Pending>,
    last_report_id: Option<String>,
}
fn read(root: &Path) -> Result<State, String> {
    let path = root.join("boot-diagnostic.state");
    if !path.exists() {
        return Ok(State::default());
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() > 65536 || metadata.file_type().is_symlink() {
        return Err("État du diagnostic au démarrage invalide.".into());
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|_| "État du diagnostic au démarrage illisible.".into())
}
fn write(root: &Path, state: &State) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let path = root.join("boot-diagnostic.state");
    let temporary = root.join(format!("boot-{}.tmp", crate::domain::new_id()));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(state).map_err(|e| e.to_string())?)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}
pub fn status(root: &Path) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        let _ = root;
        let linux = crate::linux_boot::status()?;
        Ok(
            json!({"supported":true,"pending":linux["pending"],"requested_at":linux["requested_at"],"last_report_id":null,"platform":"linux","linux":linux}),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let state = read(root)?;
        Ok(
            json!({"supported":cfg!(target_os="windows"),"pending":state.pending.is_some(),"requested_at":state.pending.map(|p|p.requested_at),"last_report_id":state.last_report_id,"platform":std::env::consts::OS}),
        )
    }
}
pub fn forget(root: &Path) -> Result<Value, String> {
    let mut state = read(root)?;
    state.pending = None;
    write(root, &state)?;
    status(root)
}
pub fn prepare(service: &Arc<AppService>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if service.active_job().is_some_and(|job| job.running) {
            return Err(
                "Terminez le diagnostic en cours avant de préparer un test au démarrage.".into(),
            );
        }
        crate::linux_boot::request("schedule").map(|_| ())
    }
    #[cfg(not(target_os = "linux"))]
    {
        if !cfg!(target_os = "windows") {
            return Err(
                "La planification via le diagnostic système n’est disponible que sous Windows."
                    .into(),
            );
        }
        if service.active_job().is_some_and(|j| j.running) {
            return Err(
                "Terminez le diagnostic en cours avant de préparer un test au démarrage.".into(),
            );
        }
        let root = service.data_root();
        let mut state = read(&root)?;
        if state.pending.is_none() {
            state.pending = Some(Pending {
                requested_at: crate::domain::timestamp_ms(),
                machine_id: service.inventory(false).machine_id,
            });
            write(&root, &state)?;
        }
        Ok(())
    }
}
pub fn collect(service: &Arc<AppService>) -> Result<Value, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = service;
        Ok(
            json!({"imported":false,"reason":"Résultats système automatiques disponibles sous Windows uniquement."}),
        )
    }
    #[cfg(target_os = "windows")]
    {
        use crate::domain::{Report, RunOptions, summarize};
        let root = service.data_root();
        let mut state = read(&root)?;
        let Some(pending) = state.pending.as_ref() else {
            return Ok(json!({"imported":false}));
        };
        if service.active_job().is_some_and(|j| j.running) {
            return Ok(json!({"imported":false,"reason":"Un diagnostic est en cours."}));
        }
        let script = r#"[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); $items=@(Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Microsoft-Windows-MemoryDiagnostics-Results'} -MaxEvents 32 -ErrorAction Stop | ForEach-Object { $x=[xml]$_.ToXml();$d=@{};foreach($n in $x.Event.EventData.Data){if($n.Name){$d[$n.Name]=[string]$n.'#text'}};[pscustomobject]@{provider=$_.ProviderName;event_id=$_.Id;record_id=$_.RecordId;created_at=([DateTimeOffset]$_.TimeCreated).ToUnixTimeMilliseconds();completion_type=[string]$x.Event.UserData.Results.CompletionType;num_bad_pages=$d['NumBadPages'];message=$_.Message;xml=$_.ToXml()} });ConvertTo-Json -InputObject $items -Depth 6 -Compress"#;
        let output = crate::inventory::command(
            "powershell.exe",
            &[
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ],
            std::time::Duration::from_secs(10),
        )?;
        if output.code != 0 {
            return Ok(
                json!({"imported":false,"reason":"Journal système absent ou accès refusé. Aucun résultat n’est déduit."}),
            );
        }
        let events: Vec<Value> =
            serde_json::from_str(&output.stdout).map_err(|_| "Journal Windows illisible.")?;
        let candidate = events
            .iter()
            .filter(|event| {
                event["created_at"].as_u64().unwrap_or(u64::MAX)
                    <= crate::domain::timestamp_ms() + 300000
            })
            .find_map(|event| {
                parse_windows_event(event, pending.requested_at).map(|check| (event, check))
            });
        let Some((event, check)) = candidate else {
            return Ok(
                json!({"imported":false,"reason":"En attente d’un événement Windows postérieur à votre demande."}),
            );
        };
        let inventory = service.inventory(true);
        if inventory.machine_id != pending.machine_id {
            return Err("L’identité de la machine a changé. Le résultat n’a pas été associé automatiquement.".into());
        }
        let timestamp = event["created_at"].as_u64().ok_or("Horodatage absent.")?;
        let record = event["record_id"]
            .as_u64()
            .ok_or("Identifiant d’événement absent.")?;
        let report = Report {
            source: "windows_memory_diagnostic".into(),
            schema_version: 1,
            id: format!("{timestamp}-0-{record}"),
            label: "Mémoire · test Windows au démarrage".into(),
            created_at: timestamp,
            duration_ms: 0,
            completed: check.status != Status::Unavailable,
            options: RunOptions {
                components: vec!["memory".into()],
                memory_mib: 0,
                passes: 0,
                disk_mib: 0,
                cpu_seconds: 0,
                label: "Diagnostic mémoire Windows".into(),
            },
            inventory,
            summary: summarize(std::slice::from_ref(&check)),
            checks: vec![check],
        };
        service.repository.save(&report)?;
        state.pending = None;
        state.last_report_id = Some(report.id.clone());
        write(&root, &state)?;
        Ok(json!({"imported":true,"report_id":report.id}))
    }
}
