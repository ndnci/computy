use crate::{domain::Report, repository::ReportRepository};
use serde_json::{Value, json};
pub fn compare(repository: &dyn ReportRepository, ids: &[String]) -> Result<Value, String> {
    if !(2..=4).contains(&ids.len()) {
        return Err("Sélectionnez deux à quatre rapports.".into());
    }
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    if unique.len() != ids.len() {
        return Err("Les rapports doivent être distincts.".into());
    }
    let reports = ids
        .iter()
        .map(|id| {
            repository.get(id).and_then(|value| {
                serde_json::from_value::<Report>(value).map_err(|e| e.to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let baseline = &reports[0];
    let mut warnings = vec![];
    if reports
        .iter()
        .any(|r| r.inventory.machine_id != baseline.inventory.machine_id)
    {
        warnings.push("Machines différentes ou identité système modifiée.");
    }
    let protocol = |report: &Report| {
        let mut options = report.options.clone();
        options.label.clear();
        options.components.sort();
        json!({"source":report.source,"options":options})
    };
    if reports.iter().any(|r| protocol(r) != protocol(baseline)) {
        warnings.push("Protocoles différents ; performances non directement comparables.");
    }
    if reports.iter().any(|r| !r.completed) {
        warnings.push("Au moins un diagnostic est incomplet.");
    }
    let deltas=reports.iter().map(|r|json!({"id":r.id,"memory_bytes_delta":r.inventory.memory.total_bytes as i128-baseline.inventory.memory.total_bytes as i128,"duration_ms_delta":r.duration_ms as i128-baseline.duration_ms as i128})).collect::<Vec<_>>();
    Ok(
        json!({"baseline_id":baseline.id,"warnings":warnings,"deltas":deltas,"reports":reports,"interpretation":"Un gain de capacité ou de performance n’est pas une preuve de meilleure santé."}),
    )
}
