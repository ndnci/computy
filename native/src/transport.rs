use crate::{domain::RunOptions, service::AppService};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn dispatch(service: &Arc<AppService>, method: &str, params: &Value) -> Result<Value, String> {
    let id = || {
        params["id"]
            .as_str()
            .ok_or_else(|| "Identifiant requis.".to_owned())
    };
    match method {
        "clean_sources" => Ok(crate::clean::sources()),
        "clean_preview" => Ok(json!(crate::clean::preview(
            &service.data_root(),
            serde_json::from_value(params.clone()).map_err(|e| e.to_string())?
        )?)),
        "clean_execute" => {
            if service.active_job().is_some_and(|j| j.running) {
                return Err("Stop the diagnostic before cleaning".into());
            }
            let excluded: Vec<usize> =
                serde_json::from_value(params.get("excluded").cloned().unwrap_or(json!([])))
                    .map_err(|e| e.to_string())?;
            Ok(json!(crate::clean::execute(
                &service.data_root(),
                id()?,
                &excluded,
                params["confirmed"] == true
            )?))
        }
        "clean_schedules" => Ok(json!(crate::clean::schedules(&service.data_root())?)),
        "clean_schedule" => Ok(json!(crate::clean::schedule(
            &service.data_root(),
            id()?,
            params["interval_hours"]
                .as_u64()
                .ok_or("Interval required")?,
            params["max_bytes"].as_u64().ok_or("Byte limit required")?,
            params["confirmed"] == true
        )?)),
        "clean_unschedule" => {
            crate::clean::remove_schedule(&service.data_root(), id()?)?;
            Ok(json!(true))
        }
        "clean_run_due" => {
            if service.active_job().is_some_and(|j| j.running) {
                return Err("A diagnostic is active".into());
            }
            Ok(json!(crate::clean::run_due(&service.data_root())?))
        }
        "applications" => Ok(json!(crate::applications::list())),
        "application_action" => crate::applications::action(
            id()?,
            params["action"].as_str().ok_or("Action required")?,
            params["remove_data"] == true,
            params["confirmed"] == true,
        ),
        "fans" => Ok(json!(crate::fans::inventory())),
        "monitor" => Ok(json!(service.monitor(true))),
        "inventory" => Ok(json!(
            service.inventory(params["refresh"].as_bool().unwrap_or(false))
        )),
        "history" => Ok(json!(service.repository.list()?)),
        "report" => service.repository.get(id()?),
        "start" => {
            let preferences = crate::preferences::load(&service.data_root())?;
            let mut effective = json!(RunOptions {
                memory_mib: preferences.memory_mib,
                passes: preferences.passes,
                disk_mib: preferences.disk_mib,
                cpu_seconds: preferences.cpu_seconds,
                ..RunOptions::default()
            });
            let requested = params.as_object().ok_or("Options invalides.")?;
            effective.as_object_mut().unwrap().extend(requested.clone());
            let options: RunOptions = serde_json::from_value(effective)
                .map_err(|e| format!("Options invalides : {e}"))?;
            Ok(json!({"id":service.start(options)?}))
        }
        "job" => Ok(json!(service.job(id()?)?)),
        "active" => Ok(json!(service.active_job())),
        "cancel" => {
            service.cancel(id()?)?;
            Ok(json!(true))
        }
        "rename" => {
            service
                .repository
                .rename(id()?, params["label"].as_str().ok_or("Nom requis.")?)?;
            Ok(json!(true))
        }
        "delete" => {
            service.repository.delete(id()?)?;
            Ok(json!(true))
        }
        "offline_status" => crate::offline::status(&service.data_root()),
        "offline_collect" => crate::offline::collect(service),
        "offline_forget" => crate::offline::forget(&service.data_root()),
        "preferences" => Ok(json!(crate::preferences::load(&service.data_root())?)),
        "save_preferences" => {
            let preferences = serde_json::from_value(params.clone())
                .map_err(|e| format!("Préférences invalides : {e}"))?;
            crate::preferences::save(&service.data_root(), &preferences)?;
            Ok(json!(preferences))
        }
        "compare" => {
            let ids: Vec<String> = serde_json::from_value(params["ids"].clone())
                .map_err(|_| "Liste d’identifiants requise.".to_owned())?;
            crate::comparison::compare(service.repository.as_ref(), &ids)
        }
        _ => Err("Commande non autorisée.".into()),
    }
}
