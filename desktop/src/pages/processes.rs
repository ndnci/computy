//! Live process list (former pages/Processes.tsx).
use crate::{
    app::Computy,
    components::{self, NoticeTone, notice, spinner},
    forms::{option, select},
    i18n::{tp, tx},
    intl,
    model::{MonitorSnapshot, ProcessSample},
    theme::{self, c, hex},
    ui::{icon, one_line, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::{Input, InputEvent, InputState};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf, time::Duration};

pub struct ProcessesState {
    pub snapshot: Option<MonitorSnapshot>,
    pub problem: String,
    pub query: Entity<InputState>,
    pub metric: String,
    pub icons: HashMap<u32, Option<PathBuf>>,
    _subscription: Subscription,
    _poll: Task<()>,
}

impl ProcessesState {
    pub fn new(metric: Option<String>, window: &mut Window, cx: &mut Context<Computy>) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder(tx("Nom du processus ou PID…")));
        let subscription = cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        // Poll every two seconds while the page exists; the task stops when the
        // state is dropped (page change).
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok(backend) = this.read_with(cx, |this, _| this.backend.clone()) else {
                    break;
                };
                let fixture = backend.is_fixture();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let snapshot = backend.typed::<MonitorSnapshot>("monitor", json!({}));
                        let icons: HashMap<u32, Option<PathBuf>> = match (&snapshot, fixture) {
                            (Ok(snapshot), false) => snapshot
                                .processes
                                .iter()
                                .map(|p| (p.pid, crate::platform::process_icon(p.pid)))
                                .collect(),
                            _ => HashMap::new(),
                        };
                        (snapshot, icons)
                    })
                    .await;
                let updated = this.update(cx, |this, cx| {
                    if let Some(state) = this.pages.processes.as_mut() {
                        match result.0 {
                            Ok(snapshot) => {
                                state.snapshot = Some(snapshot);
                                state.problem.clear();
                            }
                            Err(error) => {
                                state.problem = error;
                                state.snapshot = None;
                            }
                        }
                        state.icons = result.1;
                        cx.notify();
                    }
                });
                if updated.is_err() || fixture {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        });
        Self {
            snapshot: None,
            problem: String::new(),
            query,
            metric: metric
                .filter(|m| ["memory", "cpu", "gpu", "storage"].contains(&m.as_str()))
                .unwrap_or_else(|| "memory".into()),
            icons: HashMap::new(),
            _subscription: subscription,
            _poll: poll,
        }
    }
}

/// Process icon resolved locally, or the generic application glyph.
pub fn avatar(icon_path: Option<&PathBuf>) -> impl IntoElement {
    div()
        .size(px(29.))
        .flex_none()
        .rounded(px(8.))
        .bg(c((0xf7f9f6, 0x253324)))
        .border_1()
        .border_color(c(theme::BORDER))
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .child(match icon_path {
            Some(path) => img(path.clone()).size(px(20.)).into_any_element(),
            None => icon("app-window", 17., "2", hex(0x6d7c70)).into_any_element(),
        })
}

const COLUMNS: [(&str, f32); 9] = [
    ("PID", 75.),
    ("Processus", 324.),
    ("RAM", 106.),
    ("RAM %", 79.),
    ("CPU %", 77.),
    ("Lecture / s", 117.),
    ("Écriture / s", 122.),
    ("Part E/S", 94.),
    ("Mémoire GPU", 137.),
];

fn cell(index: usize) -> Div {
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .map(move |mut d| {
            d.style().flex_grow = Some(COLUMNS[index].1);
            d
        })
        .min_w_0()
        .pl(px(if index == 0 { 22. } else { 12. }))
        .pr(px(if index == 8 { 16. } else { 12. }))
}

pub fn render(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.processes.is_none() {
        let metric = app.pages.requested_metric.take();
        let state = ProcessesState::new(metric, window, cx);
        app.pages.processes = Some(state);
    }
    let locale = crate::i18n::locale();
    let state = app.pages.processes.as_ref().unwrap();
    let query = state.query.read(cx).value().to_lowercase();
    let metric = state.metric.clone();
    let mut processes: Vec<ProcessSample> = state
        .snapshot
        .as_ref()
        .map(|s| s.processes.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            format!("{} {}", p.name, p.pid)
                .to_lowercase()
                .contains(&query)
        })
        .collect();
    processes.sort_by(|a, b| {
        b.value(&metric)
            .unwrap_or(-1.)
            .partial_cmp(&a.value(&metric).unwrap_or(-1.))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pid.cmp(&b.pid))
    });
    let percent = |n: Option<f64>| {
        n.map_or("—".to_owned(), |v| {
            tp(
                "{{value}} %",
                &[("value", ((v * 10.).round() / 10.).into())],
            )
        })
    };
    let bytes = |n: Option<f64>| n.map_or("—".to_owned(), |v| intl::bytes(locale, Some(v), 1));
    let count = processes.len();
    let rows = processes.iter().enumerate().map(|(i, p)| {
        let values = [
            bytes(p.memory_bytes),
            percent(p.memory_percent),
            percent(p.cpu_percent),
            bytes(p.read_bytes_per_second),
            bytes(p.written_bytes_per_second),
            percent(p.io_percent),
            bytes(p.gpu_memory_bytes),
        ];
        div()
            .flex()
            .items_center()
            .h(px(64.))
            .when(i + 1 < count, |d| {
                d.border_b_1().border_color(c(theme::BORDER))
            })
            .text_size(px(12.))
            .text_color(c(theme::TEXT))
            .child(cell(0).child(p.pid.to_string()))
            .child(
                cell(1).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(avatar(state.icons.get(&p.pid).and_then(Option::as_ref)))
                        .child(
                            one_line(
                                if p.name.is_empty() {
                                    tx("Nom indisponible")
                                } else {
                                    p.name.clone()
                                },
                                12.,
                                400.,
                                c(theme::TEXT),
                            )
                            .flex_1(),
                        ),
                ),
            )
            .children(
                values
                    .into_iter()
                    .enumerate()
                    .map(|(j, v)| cell(j + 2).child(v)),
            )
            .into_any_element()
    });
    let query_entity = state.query.clone();
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("EN DIRECT"),
            "Ce qui utilise votre ordinateur.",
            Some("Tous les processus accessibles, sans limite de nombre. Recherchez un nom ou un PID pour les retrouver."),
            None,
        ))
        .child(notice(NoticeTone::Info, vec![tx("CPU : part de la capacité totale du processeur. RAM : mémoire résidente et part de la RAM physique ; les pages partagées peuvent être comptées plusieurs fois. Les processus protégés peuvent masquer leurs mesures.").into_any_element()]).mb(px(18.)))
        .child(notice(NoticeTone::Info, vec![tx("Disques : les débits par processus concernent tous les volumes ; le pourcentage est la part des entrées/sorties observables, pas le taux d’occupation d’un disque. GPU : la mémoire des processus de calcul NVIDIA est affichée quand le pilote la fournit ; cette liste ne couvre pas tous les processus graphiques et ne mesure pas leur charge GPU.").into_any_element()]).mb(px(18.)))
        .when(!state.problem.is_empty(), |d| d.child(notice(NoticeTone::Warn, vec![state.problem.clone().into_any_element()]).mb(px(18.))))
        .when(state.snapshot.is_none() && state.problem.is_empty(), |d| {
            d.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .mb(px(18.))
                    .child(spinner(18., c(theme::MUTED)))
                    .child(text(tx("Lecture des processus…"), 12., 400., c(theme::MUTED))),
            )
        })
        .child(
            panel()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(15.))
                        .py(px(18.))
                        .px(px(22.))
                        .border_b_1()
                        .border_color(c(theme::BORDER))
                        .child(
                            div()
                                .w(px(350.))
                                .h(px(40.))
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .px(px(11.))
                                .bg(c((0xf7f9f6, 0x202d23)))
                                .border_1()
                                .border_color(c((0xe6ecde, 0x425435)))
                                .rounded(px(7.))
                                .text_size(px(13.))
                                .child(icon("search", 17., "2", c((0xb0bd9f, 0xb1c699))))
                                .child(div().flex_1().child(Input::new(&query_entity).appearance(false))),
                        )
                        .child(
                            div().w(px(154.)).child(
                                select(
                                    "process-metric",
                                    &metric,
                                    vec![
                                        option("memory", tx("Mémoire vive")),
                                        option("cpu", "CPU"),
                                        option("gpu", tx("Mémoire GPU")),
                                        option("storage", tx("Activité disque")),
                                    ],
                                    app,
                                    cx,
                                    |this, value, _, _| {
                                        if let Some(state) = this.pages.processes.as_mut() {
                                            state.metric = value;
                                        }
                                    },
                                )
                                .h(px(45.))
                                .rounded(px(8.))
                                .border_color(c((0xe8ece7, 0x425435))),
                            ),
                        )
                        .child(div().flex_1())
                        .child(text(tp("Processus affichés : {{count}}", &[("count", count.into())]), 10., 400., hex(0xa5b097))),
                )
                .child(
                    div()
                        .flex()
                        .h(px(44.))
                        .items_center()
                        .bg(c((0xfbfcf8, 0x253322)))
                        .border_b_1()
                        .border_color(c(theme::BORDER))
                        .children(COLUMNS.iter().enumerate().map(|(i, (label, _))| {
                            cell(i).child(spaced(tx(label).to_uppercase(), 10., 500., hex(0xa6b299), 0.5, 13.))
                        })),
                )
                .children(rows)
                .when(state.snapshot.is_some() && count == 0, |d| {
                    d.child(div().p(px(22.)).child(text(tx("Aucun processus ne correspond à la recherche."), 12., 400., c(theme::MUTED))))
                }),
        )
        .into_any_element()
}
