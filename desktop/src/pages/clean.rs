//! Cleanup: scope, rules, mandatory preview, confirmation and schedules
//! (former pages/Clean.tsx and pages/RuleBuilder.tsx).
use crate::{
    app::Computy,
    components::{self, ButtonKind, button, button_foreground, button_with, spinner},
    forms::{option, select},
    i18n::{tp, tx},
    intl,
    theme::{self, c, hex},
    ui::{icon, one_line, panel, spaced, text},
};
use chrono::{Local, NaiveDateTime, TimeZone};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::{Input, InputEvent, InputState};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct FileItem {
    pub path: String,
    pub bytes: f64,
    pub mime: String,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct DockerItem {
    pub kind: String,
    pub id: String,
    pub label: String,
    pub bytes: f64,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct Plan {
    pub id: String,
    pub policy: Value,
    pub files: Vec<FileItem>,
    pub docker: Vec<DockerItem>,
    pub total_bytes: f64,
    pub warnings: Vec<String>,
    pub truncated: bool,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct Outcome {
    pub deleted: u64,
    pub skipped: u64,
    pub deleted_bytes: f64,
    pub errors: Vec<String>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct Schedule {
    pub id: String,
    pub policy: Value,
    pub interval_hours: u64,
    pub max_bytes: f64,
    pub next_run: f64,
    pub last_result: Option<Outcome>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CleanModal {
    Delete,
    Schedule,
}

pub struct CleanState {
    roots: Entity<InputState>,
    exclude: Entity<InputState>,
    node_modules: bool,
    docker: bool,
    docker_volumes: bool,
    docker_available: bool,
    rule: Value,
    inputs: HashMap<String, Entity<InputState>>,
    plan: Option<Plan>,
    excluded: Vec<usize>,
    busy: &'static str,
    pub modal: Option<CleanModal>,
    outcome: Option<Outcome>,
    schedules: Vec<Schedule>,
    hours: Entity<InputState>,
    limit: Entity<InputState>,
    visible: usize,
    subscriptions: Vec<Subscription>,
}

const FIELDS: [(&str, &str); 11] = [
    ("older_days", "Ancienneté (jours)"),
    ("name", "Nom exact"),
    ("extension", "Extension"),
    ("path_regex", "Chemin (regex)"),
    ("mime", "Type MIME"),
    ("min_size", "Taille minimum (octets)"),
    ("max_size", "Taille maximum (octets)"),
    ("modified_before", "Modifié avant"),
    ("modified_after", "Modifié après"),
    ("created_before", "Créé avant"),
    ("created_after", "Créé après"),
];

fn state(app: &mut Computy) -> &mut CleanState {
    app.pages.clean.as_mut().expect("clean state")
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn invalidate(app: &mut Computy) {
    let s = state(app);
    s.plan = None;
    s.outcome = None;
}

impl CleanState {
    pub fn new(window: &mut Window, cx: &mut Context<Computy>) -> Self {
        let multi = |window: &mut Window, cx: &mut Context<Computy>| {
            cx.new(|cx| InputState::new(window, cx).multi_line(true).rows(3))
        };
        let roots = multi(window, cx);
        let exclude = multi(window, cx);
        let hours = cx.new(|cx| InputState::new(window, cx).default_value("168"));
        let limit = cx.new(|cx| InputState::new(window, cx).default_value("1024"));
        let mut subscriptions = vec![];
        for input in [&roots, &exclude] {
            subscriptions.push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    invalidate(this);
                    cx.notify();
                }
            }));
        }
        for input in [&hours, &limit] {
            subscriptions.push(cx.subscribe(input, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }));
        }
        Self {
            roots,
            exclude,
            node_modules: false,
            docker: false,
            docker_volumes: false,
            docker_available: false,
            rule: json!({"op": "all", "rules": [{"op": "older_days", "value": 7}]}),
            inputs: HashMap::new(),
            plan: None,
            excluded: vec![],
            busy: "",
            modal: None,
            outcome: None,
            schedules: vec![],
            hours,
            limit,
            visible: 100,
            subscriptions,
        }
    }

    fn policy(&self, cx: &App) -> Value {
        json!({
            "roots": lines(&self.roots.read(cx).value()),
            "exclude": lines(&self.exclude.read(cx).value()),
            "rule": self.rule,
            "node_modules": self.node_modules,
            "docker": self.docker,
            "docker_volumes": self.docker_volumes,
        })
    }
}

fn reload_schedules(app: &mut Computy, cx: &mut Context<Computy>) {
    app.call(
        "clean_schedules",
        json!({}),
        cx,
        |this, result: Result<Vec<Schedule>, String>, cx| match result {
            Ok(schedules) => state(this).schedules = schedules,
            Err(error) => this.notify(error, true, cx),
        },
    );
}

fn load(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) {
    let handle = window.window_handle();
    app.call(
        "clean_sources",
        json!({}),
        cx,
        move |this, result: Result<Value, String>, cx| match result {
            Ok(data) => {
                let roots: Vec<String> = data["roots"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect();
                state(this).docker_available = data["docker_available"] == true;
                let input = state(this).roots.clone();
                if input.read(cx).value().is_empty() {
                    let _ = handle.update(cx, |_, window, cx| {
                        input.update(cx, |input, cx| {
                            input.set_value(roots.join("\n"), window, cx)
                        });
                    });
                }
            }
            Err(error) => this.notify(error, true, cx),
        },
    );
    reload_schedules(app, cx);
}

/// Rule addressed by a dotted path of child indices ("0.2").
fn rule_at<'a>(rule: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut current = rule;
    for part in path.split('.').filter(|p| !p.is_empty()) {
        current = if part == "n" {
            current.get_mut("rule")?
        } else {
            current
                .get_mut("rules")?
                .get_mut(part.parse::<usize>().ok()?)?
        };
    }
    Some(current)
}

fn update_rule(app: &mut Computy, path: &str, structural: bool, change: impl FnOnce(&mut Value)) {
    let s = state(app);
    if let Some(rule) = rule_at(&mut s.rule, path) {
        change(rule);
    }
    if structural {
        s.inputs.clear();
    }
    invalidate(app);
}

fn display_value(op: &str, value: &Value) -> String {
    if op.starts_with("modified_") || op.starts_with("created_") {
        return value
            .as_i64()
            .filter(|v| *v != 0)
            .and_then(|ms| Local.timestamp_millis_opt(ms).single())
            .map(|t| t.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default();
    }
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn parse_value(op: &str, text: &str) -> Value {
    if op.starts_with("modified_") || op.starts_with("created_") {
        return NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M")
            .ok()
            .and_then(|t| Local.from_local_datetime(&t).single())
            .map_or(json!(0), |t| json!(t.timestamp_millis()));
    }
    if op.contains("size") || op.contains("days") {
        return json!(text.trim().parse::<u64>().unwrap_or(0));
    }
    json!(text)
}

fn leaf_input(
    app: &mut Computy,
    path: &str,
    op: &str,
    value: &Value,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> Entity<InputState> {
    if let Some(input) = state(app).inputs.get(path) {
        return input.clone();
    }
    let initial = display_value(op, value);
    let input = cx.new(|cx| InputState::new(window, cx).default_value(initial));
    let key = path.to_owned();
    let subscription = cx.subscribe(&input, move |this, input, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            let text = input.read(cx).value().to_string();
            let s = state(this);
            if let Some(rule) = rule_at(&mut s.rule, &key) {
                let op = rule["op"].as_str().unwrap_or_default().to_owned();
                rule["value"] = parse_value(&op, &text);
            }
            invalidate(this);
            cx.notify();
        }
    });
    let s = state(app);
    s.subscriptions.push(subscription);
    s.inputs.insert(path.to_owned(), input.clone());
    input
}

fn field_box(child: impl IntoElement) -> Div {
    div()
        .h(px(40.))
        .flex()
        .items_center()
        .px(px(12.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(9.))
        .text_size(px(13.))
        .child(child)
}

fn text_small_button(id: SharedString, label: &str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .min_h(px(32.))
        .py(px(8.))
        .px(px(11.))
        .rounded(px(8.))
        .cursor_pointer()
        .child(text(tx(label), 11., 600., c(theme::TEXT)).line_height(px(13.2)))
}

fn rule_view(
    app: &mut Computy,
    rule: Value,
    path: String,
    depth: usize,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> AnyElement {
    let secondary = button_foreground(ButtonKind::Secondary);
    if let Some(children) = rule.get("rules").and_then(Value::as_array).cloned() {
        let op = rule["op"].as_str().unwrap_or("all").to_owned();
        let count = children.len();
        let p = path.clone();
        let add_condition = path.clone();
        let add_group = path.clone();
        let rows: Vec<AnyElement> = children
            .into_iter()
            .enumerate()
            .map(|(index, child)| {
                let child_path = if path.is_empty() {
                    index.to_string()
                } else {
                    format!("{path}.{index}")
                };
                let remove = path.clone();
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(div().flex_1().min_w_0().child(rule_view(
                        app,
                        child,
                        child_path.clone(),
                        depth + 1,
                        window,
                        cx,
                    )))
                    .child(
                        div()
                            .id(SharedString::from(format!("rule-remove-{child_path}")))
                            .size(px(30.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(7.))
                            .cursor_pointer()
                            .child(icon("trash-2", 15., "2", hex(0x6d7c70)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                update_rule(this, &remove, true, |rule| {
                                    if let Some(list) = rule["rules"].as_array_mut()
                                        && index < list.len()
                                    {
                                        list.remove(index);
                                    }
                                });
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        return div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(14.))
            .bg(c((0xf6f8f6, 0x17221b)))
            .border_1()
            .border_color(c(theme::BORDER))
            .rounded(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div().w(px(206.)).child(
                            select(
                                Box::leak(format!("rule-op-{p}").into_boxed_str()),
                                &op,
                                vec![option("all", tx("Toutes les conditions (ET)")), option("any", tx("Une condition au moins (OU)"))],
                                app,
                                cx,
                                move |this, value, _, _| update_rule(this, &p, false, |rule| rule["op"] = json!(value)),
                            )
                            .rounded(px(9.))
                            .border_color(c(theme::BORDER)),
                        ),
                    )
                    .child(div().flex_1())
                    .child(
                        button_with(SharedString::from(format!("add-condition-{add_condition}")), ButtonKind::Secondary, true, Some(icon("plus", 14., "2", secondary).into_any_element()), tx("Condition"), count >= 16)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if count < 16 {
                                    update_rule(this, &add_condition, true, |rule| {
                                        if let Some(list) = rule["rules"].as_array_mut() {
                                            list.push(json!({"op": "extension", "value": "log"}));
                                        }
                                    });
                                    cx.notify();
                                }
                            })),
                    )
                    .when(depth < 6, |d| {
                        d.child(
                            button_with(SharedString::from(format!("add-group-{add_group}")), ButtonKind::Secondary, true, Some(icon("plus", 14., "2", secondary).into_any_element()), tx("Groupe"), false)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    update_rule(this, &add_group, true, |rule| {
                                        if let Some(list) = rule["rules"].as_array_mut() {
                                            list.push(json!({"op": "any", "rules": [{"op": "name", "value": ""}]}));
                                        }
                                    });
                                    cx.notify();
                                })),
                        )
                    }),
            )
            .children(rows)
            .into_any_element();
    }
    if let Some(inner) = rule.get("rule").cloned() {
        let p = path.clone();
        let inner_path = if path.is_empty() {
            "n".to_owned()
        } else {
            format!("{path}.n")
        };
        return div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(text(tx("Exclure si"), 12., 700., c(theme::TEXT)))
            .child(div().flex_1().min_w_0().child(rule_view(
                app,
                inner,
                inner_path,
                depth + 1,
                window,
                cx,
            )))
            .child(
                text_small_button(SharedString::from(format!("unnegate-{p}")), "Annuler").on_click(
                    cx.listener(move |this, _, _, cx| {
                        update_rule(this, &p, true, |rule| {
                            let inner = rule["rule"].take();
                            *rule = inner;
                        });
                        cx.notify();
                    }),
                ),
            )
            .into_any_element();
    }
    let op = rule["op"].as_str().unwrap_or("older_days").to_owned();
    let input = leaf_input(app, &path, &op, &rule["value"], window, cx);
    let p = path.clone();
    let negate = path.clone();
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(
            div().w(px(179.)).child(
                select(
                    Box::leak(format!("rule-field-{p}").into_boxed_str()),
                    &op,
                    FIELDS.iter().map(|(id, label)| option(*id, tx(label))).collect(),
                    app,
                    cx,
                    move |this, value, _, _| {
                        update_rule(this, &p, true, |rule| {
                            let numeric = ["size", "days", "before", "after"].iter().any(|k| value.contains(k));
                            *rule = json!({"op": value, "value": if numeric { json!(0) } else { json!("") }});
                        })
                    },
                )
                .rounded(px(9.))
                .border_color(c(theme::BORDER)),
            ),
        )
        .child(field_box(Input::new(&input).appearance(false)).flex_1().min_w_0())
        .child(text_small_button(SharedString::from(format!("negate-{negate}")), "Exclure si").on_click(cx.listener(move |this, _, _, cx| {
            update_rule(this, &negate, true, |rule| {
                let inner = rule.take();
                *rule = json!({"op": "not", "rule": inner});
            });
            cx.notify();
        })))
        .into_any_element()
}

fn native_check(checked: bool, disabled: bool) -> impl IntoElement {
    crate::forms::native_checkbox(checked, disabled, hex(crate::forms::BROWSER_ACCENT))
}

fn table_check(checked: bool) -> impl IntoElement {
    crate::forms::native_checkbox(checked, false, c(theme::GREEN))
}

fn utility_check(
    id: &'static str,
    checked: bool,
    disabled: bool,
    label: &str,
    detail: String,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(3.))
        .when(!disabled, |d| d.cursor_pointer())
        .child(native_check(checked, disabled))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(text(tx(label), 12., 600., c(theme::TEXT)).line_height(px(19.2)))
                .child(text(detail, 11., 400., hex(0x6d7c70)).line_height(px(17.6))),
        )
}

fn heading(icon_name: &str, size: f32, title: &str, body: String) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap(px(14.))
        .child(icon(icon_name, size, "2", c(theme::GREEN)).mt(px(3.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(spaced(tx(title), 17., 650., c(theme::HEADING), -0.4, 21.).family("Sora"))
                .child(
                    text(body, 12., 400., hex(0x6d7c70))
                        .line_height(px(20.4))
                        .mt(px(6.)),
                ),
        )
}

pub fn render(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.clean.is_none() {
        let state = CleanState::new(window, cx);
        app.pages.clean = Some(state);
        load(app, window, cx);
    }
    let locale = crate::i18n::locale();
    let bytes = |v: f64| intl::bytes(locale, Some(v), 1);
    let rule = state(app).rule.clone();
    let rules = rule_view(app, rule, String::new(), 0, window, cx);
    let s = app.pages.clean.as_ref().unwrap();
    let busy = s.busy;
    let has_plan = s.plan.is_some();
    let roots_empty = lines(&s.roots.read(cx).value()).is_empty();
    let can_preview = busy.is_empty() && (!roots_empty || s.docker);
    let primary = button_foreground(ButtonKind::Primary);
    let secondary = button_foreground(ButtonKind::Secondary);
    let step = |icon_name: &str, label: String, active: bool| {
        let color = if active {
            c(theme::GREEN)
        } else {
            hex(0x6d7c70)
        };
        div()
            .flex()
            .items_center()
            .gap(px(9.))
            .child(icon(icon_name, 16., "2", color))
            .child(text(label, 12., if active { 700. } else { 400. }, color))
    };
    let column = |label: &str, input: &Entity<InputState>| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(9.))
            .child(text(tx(label), 12., 600., c(theme::TEXT)))
            .child(
                div()
                    .h(px(83.))
                    .py(px(10.))
                    .px(px(12.))
                    .bg(c(theme::PANEL))
                    .border_1()
                    .border_color(c(theme::BORDER))
                    .rounded(px(9.))
                    .font_family("monospace")
                    .text_size(px(12.))
                    .font_weight(FontWeight(600.))
                    .line_height(px(20.4))
                    .child(Input::new(input).appearance(false).h_full()),
            )
    };
    let (node_modules, docker, docker_volumes, docker_available) = (
        s.node_modules,
        s.docker,
        s.docker_volumes,
        s.docker_available,
    );
    let scope = panel()
        .mt(px(24.))
        .p(px(26.))
        .flex()
        .flex_col()
        .child(heading("folder", 22., "Périmètre du nettoyage", tx("Un chemin absolu par ligne. Les liens symboliques sont ignorés.")))
        .child(div().mt(px(22.)).flex().gap(px(18.)).child(column("Dossiers inclus", &s.roots)).child(column("Dossiers exclus", &s.exclude)))
        .child(
            div().mt(px(18.)).child(
                utility_check("node-modules", node_modules, !busy.is_empty(), "Uniquement les fichiers dans node_modules", tx("Indiquez vos dossiers de projets. Le code source est conservé."))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if busy.is_empty() {
                            state(this).node_modules = !node_modules;
                            invalidate(this);
                            cx.notify();
                        }
                    })),
            ),
        )
        .child(
            div()
                .mt(px(24.))
                .flex()
                .flex_col()
                .child(spaced(tx("Conditions"), 17., 650., c(theme::HEADING), -0.2, 21.).family("Sora"))
                .child(text(tx("Une date de création inconnue ne satisfait pas un critère de date."), 12., 400., hex(0x6d7c70)).line_height(px(20.4)).mt(px(10.))),
        )
        .child(div().mt(px(12.)).child(rules))
        .child(
            div().mt(px(18.)).child(
                utility_check(
                    "docker",
                    docker,
                    !busy.is_empty() || !docker_available,
                    "Docker",
                    tx(if docker_available { "Images inutilisées et conteneurs arrêtés. Sélection séparée après analyse." } else { "Docker local indisponible." }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if busy.is_empty() && docker_available {
                        let s = state(this);
                        s.docker = !docker;
                        s.docker_volumes = false;
                        invalidate(this);
                        cx.notify();
                    }
                })),
            ),
        )
        .when(docker, |d| {
            d.child(
                div().mt(px(12.)).child(
                    utility_check("docker-volumes", docker_volumes, !busy.is_empty(), "Inclure les volumes Docker inutilisés", tx("Ils peuvent contenir des bases de données. Aucune sélection automatique."))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if busy.is_empty() {
                                state(this).docker_volumes = !docker_volumes;
                                invalidate(this);
                                cx.notify();
                            }
                        })),
                ),
            )
        })
        .child(
            div()
                .mt(px(22.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(14.))
                .child(div().flex().items_center().gap(px(7.)).child(icon("shield-check", 16., "2", hex(0x6d7c70))).child(text(tx("Aperçu obligatoire avant toute suppression"), 11., 400., hex(0x6d7c70))))
                .child(
                    button_with(
                        "clean-preview",
                        ButtonKind::Primary,
                        false,
                        Some(if busy == "preview" { spinner(17., primary) } else { icon("search", 17., "2", primary).into_any_element() }),
                        tx(if busy == "preview" { "Analyse en cours…" } else { "Analyser sans supprimer" }),
                        !can_preview,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if can_preview {
                            preview(this, cx);
                        }
                    })),
                ),
        );
    let plan_panel = s.plan.clone().map(|plan| {
        let excluded = s.excluded.clone();
        let selected_bytes: f64 = plan.files.iter().enumerate().filter(|(i, _)| !excluded.contains(i)).map(|(_, f)| f.bytes).sum();
        let selected_count = plan.files.len() + plan.docker.len() - excluded.len();
        let all_files = !plan.files.is_empty() && (0..plan.files.len()).all(|i| !excluded.contains(&i));
        let file_count = plan.files.len();
        let docker_count = plan.docker.len();
        let visible = s.visible.min(file_count);
        let row = |index: usize, label: String, kind: String, size: String, checked: bool, cx: &mut Context<Computy>| {
            div()
                .id(SharedString::from(format!("plan-row-{index}")))
                .flex()
                .items_center()
                .border_t_1()
                .border_color(c(theme::BORDER))
                .cursor_pointer()
                .child(
                    div()
                        .flex_grow()
                        .flex_basis(relative(0.))
                        .map(|mut d| {
                            d.style().flex_grow = Some(755.);
                            d
                        })
                        .min_w_0()
                        .py(px(11.))
                        .px(px(13.))
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .child(table_check(checked))
                        .child(one_line(label, 12., 400., c(theme::TEXT)).flex_1()),
                )
                .child(div().flex_grow().flex_basis(relative(0.)).map(|mut d| { d.style().flex_grow = Some(237.); d }).min_w_0().py(px(11.)).px(px(13.)).child(one_line(kind, 11., 400., c(theme::TEXT))))
                .child(div().flex_grow().flex_basis(relative(0.)).map(|mut d| { d.style().flex_grow = Some(87.); d }).min_w_0().py(px(11.)).px(px(13.)).child(one_line(size, 11., 400., c(theme::TEXT))))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let s = state(this);
                    if let Some(position) = s.excluded.iter().position(|i| *i == index) {
                        s.excluded.remove(position);
                    } else {
                        s.excluded.push(index);
                    }
                    cx.notify();
                }))
                .into_any_element()
        };
        let mut rows: Vec<AnyElement> = plan.files[..visible]
            .iter()
            .enumerate()
            .map(|(i, f)| row(i, f.path.clone(), f.mime.clone(), bytes(f.bytes), !excluded.contains(&i), cx))
            .collect();
        rows.extend(plan.docker.iter().enumerate().map(|(i, item)| {
            let index = file_count + i;
            row(
                index,
                item.label.clone(),
                format!("Docker · {}", item.kind),
                if item.bytes > 0. { format!("≈ {}", bytes(item.bytes)) } else { tx("Inconnue") },
                !excluded.contains(&index),
                cx,
            )
        }));
        let header_cell = |label: &str, grow: f32| {
            div()
                .flex_grow()
                .flex_basis(relative(0.))
                .map(move |mut d| {
                    d.style().flex_grow = Some(grow);
                    d
                })
                .p(px(13.))
                .child(text(tx(label), 11., 700., hex(0x6d7c70)))
        };
        let truncated = plan.truncated;
        panel()
            .mt(px(24.))
            .p(px(26.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(spaced(tx("APERÇU · AUCUNE SUPPRESSION"), 10., 650., hex(0x899582), 1.75, 13.).mt(px(6.)))
                            .child(spaced(bytes(selected_bytes), 42., 650., c(theme::HEADING), -2., 53.).family("Sora").mt(px(14.)))
                            .child(text(tp("{{count}} éléments sélectionnés", &[("count", selected_count.into())]), 12., 400., hex(0x6d7c70)).line_height(px(19.8)).mt(px(8.))),
                    )
                    .child(icon("check", 34., "2", c(theme::GREEN))),
            )
            .child(text(tx("Taille logique des fichiers. L’espace réellement disponible dépend du système de fichiers. Les couches Docker partagées ne sont pas additionnées."), 11., 400., hex(0x6d7c70)).line_height(px(18.7)))
            .when(truncated, |d| d.child(text(tx("Analyse limitée. Réduisez le périmètre avant de continuer."), 12., 400., c(theme::WARN)).mt(px(12.))))
            .children(plan.warnings.iter().map(|w| text(w.clone(), 12., 400., c(theme::WARN)).mt(px(12.)).into_any_element()))
            .child(
                div()
                    .mt(px(25.))
                    .flex()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .id("select-files")
                            .flex()
                            .items_center()
                            .gap(px(1.))
                            .cursor_pointer()
                            .child(native_check(all_files, false))
                            .child(text(tx("Sélectionner les fichiers"), 12., 600., c(theme::TEXT)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                state(this).excluded = if all_files {
                                    (0..file_count + docker_count).collect()
                                } else {
                                    (file_count..file_count + docker_count).collect()
                                };
                                cx.notify();
                            })),
                    )
                    .child(text(tx("Docker nécessite une sélection explicite."), 11., 400., hex(0x6d7c70))),
            )
            .child(
                div()
                    .mt(px(13.))
                    .border_1()
                    .border_color(c(theme::BORDER))
                    .rounded(px(11.))
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .bg(c((0xf6f8f6, 0x17221b)))
                            .child(header_cell("Fichier / ressource", 755.))
                            .child(header_cell("Type", 237.))
                            .child(header_cell("Taille", 87.)),
                    )
                    .children(rows),
            )
            .when(visible < file_count, |d| {
                d.child(
                    div().mt(px(14.)).flex().child(button("plan-more", ButtonKind::Secondary, true, None, tx("Voir plus"), false).on_click(cx.listener(|this, _, _, cx| {
                        state(this).visible += 100;
                        cx.notify();
                    }))),
                )
            })
            .child(
                div()
                    .mt(px(22.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(14.))
                    .child(
                        button_with("automate", ButtonKind::Secondary, false, Some(icon("calendar-clock", 17., "2", secondary).into_any_element()), tx("Automatiser cette règle"), !busy.is_empty() || truncated || docker)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if busy.is_empty() && !truncated && !docker {
                                    state(this).modal = Some(CleanModal::Schedule);
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        button_with("verify-clean", ButtonKind::Primary, false, Some(icon("trash-2", 17., "2", primary).into_any_element()), tx("Vérifier et nettoyer"), !busy.is_empty() || selected_count == 0 || truncated)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if busy.is_empty() && selected_count > 0 && !truncated {
                                    state(this).modal = Some(CleanModal::Delete);
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .when(docker, |d| d.child(text(tx("Les ressources Docker restent en nettoyage manuel."), 11., 400., hex(0x6d7c70)).mt(px(12.))))
            .into_any_element()
    });
    let outcome_panel = s.outcome.clone().map(|outcome| {
        panel()
            .mt(px(24.))
            .p(px(26.))
            .flex()
            .flex_col()
            .child(heading(
                "check",
                25.,
                "Nettoyage terminé.",
                format!(
                    "{} · {} · {}",
                    bytes(outcome.deleted_bytes),
                    tp(
                        "{{count}} éléments supprimés",
                        &[("count", (outcome.deleted as f64).into())]
                    ),
                    tp(
                        "{{count}} ignorés",
                        &[("count", (outcome.skipped as f64).into())]
                    )
                ),
            ))
            .children(outcome.errors.iter().map(|e| {
                text(e.clone(), 12., 400., c(theme::WARN))
                    .mt(px(12.))
                    .into_any_element()
            }))
            .into_any_element()
    });
    let schedules = s.schedules.clone();
    div()
        .flex()
        .flex_col()
        .child(components::section_title(Some("ENTRETIEN"), "Faire de la place. Garder le contrôle.", Some("Analysez, vérifiez la sélection, puis confirmez le nettoyage."), None))
        .child(
            div()
                .mt(px(24.))
                .py(px(4.))
                .flex()
                .items_center()
                .gap(px(20.))
                .child(step("search", format!("01 · {}", tx("Analyser")), !has_plan))
                .child(icon("arrow-right", 15., "2", hex(0x6d7c70)))
                .child(step("shield-check", format!("02 · {}", tx("Vérifier")), has_plan))
                .child(icon("arrow-right", 15., "2", hex(0x6d7c70)))
                .child(step("brush-cleaning", format!("03 · {}", tx("Nettoyer")), false)),
        )
        .child(scope)
        .children(plan_panel)
        .children(outcome_panel)
        .child(
            panel()
                .mt(px(24.))
                .p(px(26.))
                .flex()
                .flex_col()
                .child(heading("calendar-clock", 23., "Automatisations", tx("Exécutées quand Computy fonctionne, y compris en arrière-plan. Les échéances manquées sont reprises à la prochaine connexion.")))
                .when(schedules.is_empty(), |d| {
                    d.child(text(tx("Créez une automatisation à partir d’un aperçu vérifié."), 11., 400., hex(0x6d7c70)).line_height(px(18.7)).mt(px(22.)))
                })
                .children(schedules.into_iter().map(|task| {
                    let id = task.id.clone();
                    let roots: Vec<String> = task.policy["roots"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_owned)).collect();
                    div()
                        .mt(px(16.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(14.))
                        .p(px(16.))
                        .border_1()
                        .border_color(c(theme::BORDER))
                        .rounded(px(11.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(text(tp("Toutes les {{hours}} heures", &[("hours", (task.interval_hours as f64).into())]), 12., 700., c(theme::TEXT)))
                                .child(text(roots.join(" · "), 11., 400., hex(0x6d7c70)))
                                .child(text(
                                    format!("{} · {} · {}", tx("Prochaine exécution"), intl::date(locale, task.next_run as i64, false), bytes(task.max_bytes)),
                                    10.,
                                    400.,
                                    hex(0x6d7c70),
                                ))
                                .when_some(task.last_result.clone(), |d, last| {
                                    d.child(text(format!("{} · {}", bytes(last.deleted_bytes), last.errors.join(" · ")), 11., 400., hex(0x6d7c70)))
                                }),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("unschedule-{id}")))
                                .size(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(7.))
                                .cursor_pointer()
                                .child(icon("trash-2", 17., "2", hex(0x6d7c70)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.call("clean_unschedule", json!({"id": id}), cx, |this, result: Result<Value, String>, cx| match result {
                                        Ok(_) => reload_schedules(this, cx),
                                        Err(error) => this.notify(error, true, cx),
                                    });
                                })),
                        )
                        .into_any_element()
                })),
        )
        .into_any_element()
}

fn preview(app: &mut Computy, cx: &mut Context<Computy>) {
    let policy = state(app).policy(cx);
    let s = state(app);
    s.busy = "preview";
    s.plan = None;
    s.outcome = None;
    app.call(
        "clean_preview",
        policy,
        cx,
        |this, result: Result<Plan, String>, cx| {
            state(this).busy = "";
            match result {
                Ok(plan) => {
                    let s = state(this);
                    s.excluded = (plan.files.len()..plan.files.len() + plan.docker.len()).collect();
                    s.visible = 100;
                    s.plan = Some(plan);
                    this.notify(tx("Aperçu prêt. Aucun fichier supprimé."), false, cx);
                }
                Err(error) => this.notify(error, true, cx),
            }
        },
    );
    cx.notify();
}

pub fn modal(
    app: &mut Computy,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> Option<AnyElement> {
    let s = app.pages.clean.as_ref()?;
    let kind = s.modal?;
    let plan = s.plan.clone()?;
    let busy = s.busy;
    let locale = crate::i18n::locale();
    let excluded = s.excluded.clone();
    let selected_bytes: f64 = plan
        .files
        .iter()
        .enumerate()
        .filter(|(i, _)| !excluded.contains(i))
        .map(|(_, f)| f.bytes)
        .sum();
    let selected_count = plan.files.len() + plan.docker.len() - excluded.len();
    let primary = button_foreground(ButtonKind::Primary);
    let close = std::rc::Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
        if busy.is_empty() {
            state(this).modal = None;
            cx.notify();
        }
    }));
    let field = |label: &str, input: &Entity<InputState>| {
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(text(tx(label), 12., 600., c(theme::TEXT)))
            .child(field_box(Input::new(input).appearance(false)))
    };
    let body = match kind {
        CleanModal::Delete => {
            let plan_id = plan.id.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(spaced(intl::bytes(locale, Some(selected_bytes), 1), 28., 650., c(theme::HEADING), -1., 36.).family("Sora"))
                .child(text(tp("{{count}} éléments sélectionnés", &[("count", selected_count.into())]), 12., 400., hex(0x6d7c70)))
                .child(text(tx("Suppression définitive. Les fichiers modifiés depuis l’aperçu seront ignorés."), 12., 400., hex(0x6d7c70)))
                .child(
                    div().flex().child(
                        button_with(
                            "confirm-clean",
                            ButtonKind::Primary,
                            false,
                            Some(if busy.is_empty() { icon("trash-2", 16., "2", primary).into_any_element() } else { spinner(16., primary) }),
                            tx("Confirmer la suppression"),
                            !busy.is_empty(),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !busy.is_empty() {
                                return;
                            }
                            let excluded = state(this).excluded.clone();
                            state(this).busy = "execute";
                            this.call(
                                "clean_execute",
                                json!({"id": plan_id, "excluded": excluded, "confirmed": true}),
                                cx,
                                |this, result: Result<Outcome, String>, cx| {
                                    let s = state(this);
                                    s.busy = "";
                                    s.plan = None;
                                    s.modal = None;
                                    match result {
                                        Ok(outcome) => {
                                            state(this).outcome = Some(outcome);
                                            this.notify(tx("Nettoyage terminé."), false, cx);
                                        }
                                        Err(error) => this.notify(error, true, cx),
                                    }
                                },
                            );
                        })),
                    ),
                )
                .into_any_element()
        }
        CleanModal::Schedule => {
            let hours: u64 = s.hours.read(cx).value().trim().parse().unwrap_or(0);
            let limit: f64 = s.limit.read(cx).value().trim().parse().unwrap_or(0.);
            let valid = busy.is_empty() && (1..=8760).contains(&hours) && limit >= 1.;
            let roots: Vec<String> = plan.policy["roots"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            let plan_id = plan.id.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(text(tx("Autoriser les prochains nettoyages de ces dossiers avec les mêmes conditions. Chaque passage refait une analyse et respecte la limite ci-dessous. La sélection de fichiers de cet aperçu ne limite pas les passages futurs."), 12., 400., hex(0x6d7c70)).line_height(px(20.4)))
                .child(text(roots.join("\n"), 11., 400., hex(0x6d7c70)))
                .child(field("Intervalle (heures)", &s.hours))
                .child(field("Limite par passage (Mio)", &s.limit))
                .child(
                    div().flex().child(
                        button_with("confirm-schedule", ButtonKind::Primary, false, (!busy.is_empty()).then(|| spinner(16., primary)), tx("Autoriser cette automatisation"), !valid).on_click(cx.listener(move |this, _, _, cx| {
                            if !valid {
                                return;
                            }
                            state(this).busy = "schedule";
                            this.call(
                                "clean_schedule",
                                json!({"id": plan_id, "interval_hours": hours, "max_bytes": (limit * 1_048_576.).round() as u64, "confirmed": true}),
                                cx,
                                |this, result: Result<Value, String>, cx| {
                                    state(this).busy = "";
                                    match result {
                                        Ok(_) => {
                                            state(this).modal = None;
                                            reload_schedules(this, cx);
                                            this.notify(tx("Automatisation enregistrée."), false, cx);
                                        }
                                        Err(error) => this.notify(error, true, cx),
                                    }
                                },
                            );
                        })),
                    ),
                )
                .into_any_element()
        }
    };
    Some(components::modal(
        "clean-modal",
        tx(if kind == CleanModal::Delete {
            "Confirmer le nettoyage"
        } else {
            "Automatiser cette règle"
        }),
        false,
        body,
        close,
        window,
    ))
}

/// Capture action: run the preview with the current policy.
pub fn capture_preview(app: &mut Computy, cx: &mut Context<Computy>) {
    if app.pages.clean.is_some() {
        preview(app, cx);
    }
}
