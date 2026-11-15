//! Engine access. The interface calls the Rust engine in-process; capture mode
//! replays a frozen fixture so visual comparisons are deterministic.
use computy_engine::{
    repository::FileRepository,
    service::{AppService, SystemHardware},
    transport,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub enum Backend {
    Live(Arc<AppService>),
    Fixture(Arc<Fixture>),
}

pub struct Fixture {
    pub now: i64,
    pub engine: serde_json::Map<String, Value>,
    pub reports: serde_json::Map<String, Value>,
    pub tauri: serde_json::Map<String, Value>,
    pub job: Option<Value>,
}

impl Backend {
    pub fn live(root: PathBuf) -> Result<Self, String> {
        let repository = Arc::new(FileRepository::new(root.join("reports"))?);
        Ok(Self::Live(AppService::new(
            repository,
            Arc::new(SystemHardware),
            root.join("scratch"),
        )))
    }

    pub fn service(&self) -> Option<&Arc<AppService>> {
        match self {
            Self::Live(service) => Some(service),
            Self::Fixture(_) => None,
        }
    }

    pub fn is_fixture(&self) -> bool {
        matches!(self, Self::Fixture(_))
    }

    /// Same request limits as the former IPC boundary.
    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let limit = if method == "clean_execute" {
            262_144
        } else {
            16_384
        };
        if serde_json::to_vec(&params)
            .map_err(|e| e.to_string())?
            .len()
            > limit
        {
            return Err("Requête trop volumineuse.".into());
        }
        match self {
            Self::Live(service) => transport::dispatch(service, method, &params),
            Self::Fixture(fixture) => fixture.call(method, &params),
        }
    }

    pub fn typed<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T, String> {
        serde_json::from_value(self.call(method, params)?).map_err(|e| e.to_string())
    }

    /// Commands that were Tauri host commands in the web interface.
    pub fn host<T: DeserializeOwned>(&self, command: &str) -> Option<T> {
        match self {
            Self::Fixture(fixture) => fixture
                .tauri
                .get(command)
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            Self::Live(_) => None,
        }
    }

    pub fn now(&self) -> i64 {
        match self {
            Self::Fixture(fixture) => fixture.now,
            Self::Live(_) => chrono::Utc::now().timestamp_millis(),
        }
    }
}

impl Fixture {
    fn call(&self, method: &str, params: &Value) -> Result<Value, String> {
        match method {
            "report" => self
                .reports
                .get(params["id"].as_str().unwrap_or_default())
                .cloned()
                .ok_or_else(|| "Rapport introuvable.".into()),
            "job" | "active" => Ok(self.job.clone().unwrap_or(Value::Null)),
            "save_preferences" => Ok(params.clone()),
            "start" => Ok(json!({"id": "visual-job"})),
            "cancel" | "rename" | "delete" | "clean_unschedule" => Ok(json!(true)),
            _ => self
                .engine
                .get(method)
                .cloned()
                .ok_or_else(|| format!("fixture: {method}")),
        }
    }
}
