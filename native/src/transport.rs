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
