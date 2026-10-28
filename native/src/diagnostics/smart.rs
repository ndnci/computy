use crate::domain::{Check, Status};
use serde_json::{Value, json};

pub fn parse_smart(value: &Value) -> Check {
    let mut result = Check::new("smart", Status::Unavailable, "État SMART indisponible.");
    result.component = "storage".into();
    result.name = "Télémétrie SMART / NVMe".into();
    result.limitations = vec![
        "SMART est un indicateur déclaré par le disque, pas une garantie contre une panne future."
            .into(),
        "Les adaptateurs USB et contrôleurs RAID peuvent masquer la télémétrie.".into(),
    ];
    let exit = value
        .pointer("/smartctl/exit_status")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    result.metrics = json!({"smartctl_exit_status":exit,"model":value.get("model_name"),
        "temperature_c":value.pointer("/temperature/current"),"power_on_hours":value.pointer("/power_on_time/hours"),
        "nvme":value.get("nvme_smart_health_information_log"),"ata_attributes":value.pointer("/ata_smart_attributes/table"),
        "self_test_log":value.get("ata_smart_self_test_log"),"nvme_self_test_log":value.get("nvme_self_test_log")});
    let health = value
        .pointer("/smart_status/passed")
        .and_then(Value::as_bool);
    let nvme = value.get("nvme_smart_health_information_log");
    let critical = nvme
        .and_then(|n| n.get("critical_warning"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let errors = nvme
        .and_then(|n| n.get("media_errors"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let wear = nvme
        .and_then(|n| n.get("percentage_used"))
        .and_then(Value::as_u64);
    let attributes = value
        .pointer("/ata_smart_attributes/table")
        .and_then(Value::as_array);
    let pending = attributes
        .map(|attrs| {
            attrs
                .iter()
                .filter(|a| [197, 198].contains(&a["id"].as_u64().unwrap_or(0)))
                .filter_map(|a| a.pointer("/raw/value").and_then(Value::as_u64))
                .sum::<u64>()
        })
        .unwrap_or(0);
    let reallocated = attributes
        .and_then(|a| a.iter().find(|a| a["id"] == 5))
        .and_then(|a| a.pointer("/raw/value"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if health == Some(false) || critical != 0 || errors > 0 || pending > 0 || exit & 0b00011000 != 0
    {
        result.status = Status::Failed;
        result.message =
            "Le disque signale une anomalie : sauvegardez vos données et examinez le rapport."
                .into();
    } else if exit & 7 != 0 {
        result.message =
            "Lecture SMART refusée ou incomplète ; vérifiez l’outil et les droits d’accès.".into();
    } else if exit & 0b11100000 != 0 || wear.is_some_and(|w| w >= 100) || reallocated > 0 {
        result.status = Status::Warning;
        result.message = "SMART signale de l’usure ou un historique d’erreurs à examiner.".into();
    } else if health == Some(true)
        || nvme.is_some_and(|n| n.get("critical_warning").and_then(Value::as_u64) == Some(0))
    {
        result.status = Status::Passed;
        result.message = "Aucune alerte dans les indicateurs SMART disponibles.".into();
    }
    if exit & 7 != 0 {
        result.limitations.push(
            "Certaines commandes SMART ont échoué ; les données de télémétrie sont incomplètes."
                .into(),
        );
    }
    result
}
