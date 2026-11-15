//! Overview (former pages/Overview.tsx).
use crate::{
    app::{Computy, Page},
    components::{
        self, ButtonKind, button, check_outcome, component_summary, mood, score, score_status,
        status_badge, text_button,
    },
    i18n::{tn, tp, tx},
    intl,
    model::*,
    theme::{self, c, hex},
    ui::{icon, one_line, panel, shadow, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};

struct Card {
    page: Page,
    name: &'static str,
    caption: &'static str,
    icon: &'static str,
    color: &'static str,
    value: String,
    detail: String,
    used: Option<f64>,
    foot: String,
}

fn cards(app: &Computy) -> Vec<Card> {
    let locale = crate::i18n::locale();
    let inventory = app.inventory.as_ref();
    let memory = inventory.map(|i| &i.memory);
    let disk = inventory.and_then(|i| {
        i.disks
            .iter()
            .find(|d| d.mount_point == "/" || d.mount_point.eq_ignore_ascii_case("c:\\"))
            .or(i.disks.first())
    });
    let bytes = |v: f64| intl::bytes(locale, Some(v), 1);
    vec![
        Card {
            page: Page::Memory,
            name: "Mémoire vive",
            caption: "RAM",
            icon: "memory-stick",
            color: "sage",
            value: memory.map_or("—".into(), |m| bytes(m.total_bytes)),
            detail: match memory {
                Some(m) if !m.modules.is_empty() => {
                    let mut kinds: Vec<&str> = vec![];
                    for module in &m.modules {
                        if !kinds.contains(&module.kind.as_str()) {
                            kinds.push(&module.kind);
                        }
                    }
                    tp(
                        "{{v0}} × {{v2}}",
                        &[
                            ("v0", m.modules.len().into()),
                            ("v2", kinds.join(" / ").as_str().into()),
                        ],
                    )
                }
                Some(_) => tx("Détails des modules non exposés"),
                None => tx("Inventaire en cours"),
            },
            used: Some(memory.map_or(0., |m| {
                if m.total_bytes > 0. {
                    1. - m.available_bytes / m.total_bytes
                } else {
                    0.
                }
            })),
            foot: memory.map_or(tx("Données du système"), |m| {
                tp(
                    "{{v0}} disponibles",
                    &[("v0", bytes(m.available_bytes).as_str().into())],
                )
            }),
        },
        Card {
            page: Page::Storage,
            name: "Stockage",
            caption: "DISQUES",
            icon: "hard-drive",
            color: "sand",
            value: disk.map_or("—".into(), |d| bytes(d.total_bytes)),
            detail: disk.map_or(tx("Aucun volume accessible"), |d| {
                tx(&format!(
                    "{} · {} · {}",
                    match d.kind.as_str() {
                        "SSD" => "SSD",
                        "HDD" => "Disque dur",
                        _ => "Volume",
                    },
                    d.file_system.to_uppercase(),
                    d.mount_point
                ))
            }),
            used: Some(disk.map_or(0., |d| 1. - d.available_bytes / d.total_bytes)),
            foot: disk.map_or(tx("Volumes montés"), |d| {
                tp(
                    "{{v0}} libres",
                    &[("v0", bytes(d.available_bytes).as_str().into())],
                )
            }),
        },
        Card {
            page: Page::Cpu,
            name: "Processeur",
            caption: "CPU",
            icon: "cpu",
            color: "lilac",
            value: match inventory {
                Some(i) if i.cpu.physical_cores.is_some() => tp(
                    "{{v0}} cœurs",
                    &[("v0", (i.cpu.physical_cores.unwrap_or(0) as f64).into())],
                ),
                Some(i) => tx(&format!("{} threads", i.cpu.logical_cores)),
                None => "—".into(),
            },
            detail: inventory
                .map(|i| i.cpu.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| tx("Identification en cours")),
            used: None,
            foot: inventory.map_or(tx("Données du système"), |i| {
                tx(&format!("{} threads · {}", i.cpu.logical_cores, i.arch))
            }),
        },
    ]
}

fn card_colors(color: &str) -> (theme::Pair, theme::Pair, theme::Pair) {
    // (icon background, icon color, usage fill)
    match color {
        "sage" => (
            (0xeef3e8, 0x304129),
            (0x7e9b64, 0xa4c580),
            (0xacbf86, 0xacbf86),
        ),
        "sand" => (
            (0xfaf3e6, 0x403a29),
            (0xb29a6a, 0xbcad80),
            (0xd3bd8a, 0xd3bd8a),
        ),
        _ => (
            (0xf1edf6, 0x3a3445),
            (0xa290b3, 0xb4a4c8),
            (0xc5b8d5, 0x877499),
        ),
    }
}

/// Conic progress ring (`.health-ring`), drawn as two annular sectors.
pub fn ring(
    size: f32,
    thickness: f32,
    fraction: f32,
    fill: Hsla,
    track: Hsla,
    track_end: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let center = bounds.center();
            let outer = size / 2.;
            let inner = outer - thickness;
            let sector = |from: f32, to: f32| {
                if to <= from {
                    return None;
                }
                let steps = ((to - from) * 160.).ceil().max(2.) as usize;
                let point_at = |radius: f32, t: f32| {
                    let angle = std::f32::consts::TAU * t - std::f32::consts::FRAC_PI_2;
                    point(
                        center.x + px(radius * angle.cos()),
                        center.y + px(radius * angle.sin()),
                    )
                };
                let mut builder = PathBuilder::fill();
                builder.move_to(point_at(outer, from));
                for i in 1..=steps {
                    builder.line_to(point_at(
                        outer,
                        from + (to - from) * i as f32 / steps as f32,
                    ));
                }
                for i in (0..=steps).rev() {
                    builder.line_to(point_at(
                        inner,
                        from + (to - from) * i as f32 / steps as f32,
                    ));
                }
                builder.close();
                builder.build().ok()
            };
            if let Some(path) = sector(0., fraction) {
                window.paint_path(path, fill);
            }
            if let Some(path) = sector(fraction, track_end) {
                window.paint_path(path, track);
            }
        },
    )
    .size(px(size))
}

fn health_ring(status: Option<Status>, score_value: Option<f64>) -> impl IntoElement {
    let dark = theme::is_dark();
    // (fill, track, fill fraction, track end) from the stylesheet's conic gradients.
    let (fill, track, fraction, end) = match status {
        Some(Status::Passed) => (hex(0x8cab67), hex(0xedf1e7), 1.0, 1.0),
        Some(Status::Warning) => {
            if dark {
                (hex(0x8da86d), hex(0x36472c), 0.75, 1.0)
            } else {
                (hex(0xa6bd7e), hex(0xedf1e7), 0.75, 1.0)
            }
        }
        Some(Status::Failed) => (hex(0xc98370), hex(0xf2e9e4), 0.30, 1.0),
        _ => {
            if dark {
                (hex(0x6b8653), hex(0x36472c), 0.24, 0.99)
            } else {
                (hex(0xdce4cf), hex(0xf0f3e9), 0.24, 0.99)
            }
        }
    };
    let mood_status = match status {
        Some(s) => score_status(score_value, s),
        None => Status::Unavailable,
    };
    let value = score_value.map_or_else(|| "—".to_owned(), tn);
    div()
        .relative()
        .size(px(124.))
        .mt(px(1.))
        .mb(px(13.))
        .child(ring(124., 7., fraction, fill, track, end))
        .child(
            div()
                .absolute()
                .top(px(7.))
                .left(px(7.))
                .size(px(110.))
                .rounded_full()
                .bg(c(theme::PANEL))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.))
                .child(mood(mood_status, if status.is_some() { 32. } else { 34. }))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .child(spaced(value, 31., 550., c(theme::TEXT), -1., 37.2))
                        .child(text(tx("/ 10"), 10., 400., hex(0xa2ac97)).ml(px(3.))),
                ),
        )
}

fn art(name: &str, width: f32, height: f32) -> Img {
    let theme = if theme::is_dark() { "dark" } else { "light" };
    img(SharedString::from(format!("art/{name}-{theme}.png")))
        .w(px(width))
        .h(px(height))
}

fn hero(app: &Computy, cx: &mut Context<Computy>) -> impl IntoElement {
    let running = app.job.as_ref().is_some_and(|j| j.running);
    let dark = theme::is_dark();
    let orbit_color = c((0xd4debc, 0x4e673c));
    let orbit = move |size: f32, opacity: f32| {
        div()
            .absolute()
            .size(px(size))
            .rounded_full()
            .border_1()
            .border_color(orbit_color)
            .opacity(opacity)
    };
    let dot = |size: f32, color: u32| {
        div()
            .absolute()
            .size(px(size))
            .rounded_full()
            .bg(hex(color))
            .border_2()
            .border_color(c((0xeaf0e1, 0x2c4028)))
    };
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .map(|mut d| {
            d.style().flex_grow = Some(1.95);
            d
        })
        .min_w_0()
        .relative()
        .min_h(px(298.))
        .flex()
        .items_center()
        .bg(c((0xeaf0e1, 0x2c4028)))
        .border_1()
        .border_color(c((0xe4eadb, 0x405531)))
        .rounded(px(18.))
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .right(px(3.))
                .top_0()
                .bottom_0()
                .w(relative(0.43))
                .flex()
                .items_center()
                .justify_center()
                .child(orbit(210., 0.6))
                .child(orbit(282., 0.6))
                .child(orbit(355., 0.35))
                .child(dot(7., 0xb7c987).top(px(36.)).left(px(49.)))
                .child(dot(5., 0xa6bd72).bottom(px(27.)).right(px(52.)))
                .child(
                    art("hero-core", 206.17, 206.17)
                        .absolute()
                        .top(px(50.92))
                        .left(px(53.81))
                        .map(|i| {
                            // Centered like the flex item it replaces.
                            i
                        }),
                )
                .child(
                    art("hero-chip-one", 137.66, 95.55)
                        .absolute()
                        .top(px(23.23))
                        .right(px(0.91)),
                )
                .child(
                    art("hero-chip-two", 134.72, 93.84)
                        .absolute()
                        .bottom(px(10.08))
                        .left(px(-23.88)),
                )
                .child(
                    art("hero-chip-three", 95.44, 95.44)
                        .absolute()
                        .bottom(px(26.28))
                        .right(px(-20.72)),
                ),
        )
        .child(
            div()
                .relative()
                .w(relative(0.65))
                .pt(px(30.))
                .pb(px(27.))
                .pl(px(29.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .child(
                            div()
                                .size(px(21.))
                                .rounded_full()
                                .bg(c((0xdce7c9, 0x31472a)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon("sparkles", 12., "2", c((0x78924e, 0xa3c17f)))),
                        )
                        .child(spaced(
                            tx(" UN PEU D’ATTENTION, BEAUCOUP DE SÉRÉNITÉ")
                                .trim()
                                .to_owned(),
                            8.,
                            650.,
                            c((0x7a8e60, 0x9eb87e)),
                            1.1,
                            10.,
                        )),
                )
                .child(
                    div()
                        .mt(px(15.))
                        .flex()
                        .flex_col()
                        .child(spaced(
                            tx("Un check-up."),
                            31.,
                            600.,
                            c((0x2e4834, 0xd6e7bd)),
                            -1.1,
                            38.75,
                        ))
                        .child(
                            div()
                                .flex()
                                .child(spaced(
                                    tx("L’esprit plus léger"),
                                    31.,
                                    600.,
                                    c((0x2e4834, 0xd6e7bd)),
                                    -1.1,
                                    38.75,
                                ))
                                .child(spaced(tx("."), 31., 600., hex(0x92a956), -1.1, 38.75)),
                        ),
                )
                .child(
                    div()
                        .mt(px(11.))
                        .text_size(px(13.))
                        .line_height(px(23.4))
                        .text_color(c((0x728264, 0xb5c7a0)))
                        .child(tx("RAM, disques, processeur : faites le point"))
                        .child(
                            div().child(
                                tx(" sur ce qui fait tourner votre quotidien.")
                                    .trim()
                                    .to_owned(),
                            ),
                        ),
                )
                .child(
                    div().mt(px(19.)).flex().child(
                        button(
                            "hero-diagnostic",
                            ButtonKind::Primary,
                            false,
                            Some("scan-line"),
                            tx(if running {
                                "Voir le diagnostic"
                            } else {
                                "Lancer un diagnostic"
                            }),
                            false,
                        )
                        .py(px(11.))
                        .px(px(14.))
                        .gap(px(10.))
                        .child(icon("arrow-up-right", 17., "2", hex(0xffffff)))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.navigate(Page::Diagnostics, window, cx)
                        })),
                    ),
                ),
        )
        .when(dark, |d| d)
}

fn health_card(app: &Computy, cx: &mut Context<Computy>) -> impl IntoElement {
    let latest = app.reports.first().cloned();
    let locale = crate::i18n::locale();
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .min_w(px(265.))
        .flex()
        .flex_col()
        .items_center()
        .pt(px(22.))
        .px(px(24.))
        .pb(px(19.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(18.))
        .shadow(vec![shadow(theme::SHADOW, 0.012, 5., 20.)])
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .mb(px(13.))
                .child(spaced(
                    tx("VOTRE DERNIER BILAN"),
                    9.,
                    600.,
                    hex(0x96a08e),
                    1.3,
                    12.,
                ))
                .child(icon("activity", 17., "2", hex(0xa1ae8e))),
        )
        .map(|d| match latest {
            Some(latest) => {
                let outcome = check_outcome(&latest.checks, latest.completed).status;
                let title = if latest.summary.status == Status::Failed {
                    "Une anomalie à examiner"
                } else if outcome == Status::Warning {
                    "Un bilan à compléter"
                } else if latest.completed {
                    "Vos contrôles sont conformes"
                } else {
                    "Un bilan interrompu"
                };
                let report = latest.clone();
                d.child(health_ring(
                    Some(latest.summary.status),
                    latest.summary.score,
                ))
                .child(spaced(tx(title), 15., 650., c(theme::HEADING), -0.2, 20.))
                .child(
                    text(
                        format!(
                            "{}{}{}{}{}",
                            tn(latest.summary.checked as f64),
                            tx("/"),
                            tn(latest.summary.total as f64),
                            tx(" contrôles exploitables · "),
                            intl::date(locale, latest.created_at as i64, true)
                        ),
                        12.,
                        400.,
                        hex(0x7b896e),
                    )
                    .line_height(px(21.6))
                    .mt(px(6.)),
                )
                .child(
                    div().mt(px(11.)).child(
                        text_button(
                            "understand",
                            tx("Comprendre mon résultat "),
                            Some(("arrow-right", 15.)),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.report = Some(report.clone());
                            cx.notify();
                        })),
                    ),
                )
            }
            None => d
                .child(health_ring(None, None))
                .child(spaced(
                    tx("Faisons connaissance."),
                    15.,
                    650.,
                    c(theme::HEADING),
                    -0.2,
                    20.,
                ))
                .child(
                    div()
                        .mt(px(6.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .text_size(px(12.))
                        .line_height(px(21.6))
                        .text_color(hex(0x7b896e))
                        .child(tx("Votre premier diagnostic révélera"))
                        .child(tx("ce que votre matériel a à vous dire.")),
                )
                .child(
                    div()
                        .mt(px(11.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .py(px(6.))
                        .px(px(9.))
                        .border_1()
                        .border_color(c((0xe6ebdf, 0x475c34)))
                        .bg(if theme::is_dark() {
                            hex(0x31472a)
                        } else {
                            hexa_none()
                        })
                        .rounded(px(5.))
                        .child(icon("clock-3", 13., "2", c((0x8e9b7d, 0xa3c17f))))
                        .child(text(
                            tx(" En attente du premier bilan").trim().to_owned(),
                            9.,
                            400.,
                            c((0x8e9b7d, 0xa3c17f)),
                        )),
                ),
        })
}

fn hexa_none() -> Hsla {
    gpui::transparent_black()
}

fn component_card(app: &Computy, card: Card, cx: &mut Context<Computy>) -> impl IntoElement {
    let (icon_bg, icon_color, fill) = card_colors(card.color);
    let latest = app.reports.first();
    let checks: Vec<Check> = latest
        .map(|l| {
            l.checks
                .iter()
                .filter(|c| c.component == card.page.component())
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let summary = component_summary(
        &latest.map(|l| l.checks.clone()).unwrap_or_default(),
        card.page.component(),
    );
    let completed = latest.is_none_or(|l| l.completed);
    let logical = app.inventory.as_ref().map_or(0, |i| i.cpu.logical_cores);
    let page = card.page;
    panel()
        .id(SharedString::from(format!("component-{}", card.caption)))
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .pt(px(18.))
        .px(px(19.))
        .pb(px(15.))
        .cursor_pointer()
        .hover(|s| s.border_color(c((0xcad9bc, 0x4d6340))))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .mb(px(15.))
                .child(
                    div()
                        .size(px(39.))
                        .flex_none()
                        .rounded(px(10.))
                        .bg(c(icon_bg))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(card.icon, 23., "1.6", c(icon_color))),
                )
                .child(spaced(tx(card.caption), 9., 600., hex(0xa5ad9e), 1., 12.))
                .child(div().flex_1())
                .child(icon("arrow-up-right", 18., "2", hex(0xadb6a4))),
        )
        .child(text(tx(card.name), 12., 400., c((0x718166, 0xadc29a))))
        .child(spaced(tx(&card.value), 26., 550., c(theme::HEADING), -0.7, 33.8).mt(px(5.)))
        .child(
            div()
                .mt(px(5.))
                .h(px(18.))
                .min_w_0()
                .overflow_hidden()
                .child(
                    one_line(tx(&card.detail), 11., 400., c((0x7b8a71, 0xadc29a)))
                        .w_full()
                        .line_height(px(18.15)),
                ),
        )
        .child(match card.used {
            Some(used) => div()
                .mt(px(17.))
                .mb(px(10.))
                .h(px(4.))
                .rounded(px(10.))
                .bg(c((0xf0f3eb, 0x39492f)))
                .overflow_hidden()
                .child(
                    div()
                        .h_full()
                        .min_w(px(1.))
                        .w(relative(used.clamp(0., 1.) as f32))
                        .rounded(px(10.))
                        .bg(c(fill)),
                )
                .into_any_element(),
            None => div()
                .mt(px(17.))
                .mb(px(10.))
                .h(px(4.))
                .flex()
                .gap(px(3.))
                .overflow_hidden()
                .children((0..16).map(|i| {
                    div()
                        .flex_1()
                        .h(px(4.))
                        .rounded(px(2.))
                        .bg(if (i as u64) < logical {
                            c(fill)
                        } else {
                            c((0xf1eef5, 0x3e3548))
                        })
                }))
                .into_any_element(),
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(7.))
                .child(text(tx(&card.foot), 10., 400., c((0x7c8e6b, 0xadc29a))).whitespace_nowrap())
                .child(status_badge(
                    summary.status,
                    true,
                    latest.map(|_| checks.as_slice()),
                    completed,
                )),
        )
        .on_click(cx.listener(move |this, _, window, cx| this.navigate(page, window, cx)))
}

fn recent_panel(app: &Computy, cx: &mut Context<Computy>) -> impl IntoElement {
    let locale = crate::i18n::locale();
    let reports: Vec<Report> = app.reports.iter().take(3).cloned().collect();
    panel()
        .flex_grow()
        .flex_basis(relative(0.))
        .map(|mut d| {
            d.style().flex_grow = Some(1.65);
            d
        })
        .min_w_0()
        .overflow_hidden()
        .child(components::panel_header(
            "Votre historique",
            Some(tx("Les petits changements méritent d’être suivis.")),
            Some(
                text_button("history-all", tx("Tout voir "), Some(("arrow-right", 15.)))
                    .on_click(cx.listener(|this, _, window, cx| this.navigate(Page::History, window, cx)))
                    .into_any_element(),
            ),
        ))
        .map(|d| {
            if reports.is_empty() {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(22.))
                        .pt(px(30.))
                        .px(px(30.))
                        .pb(px(40.))
                        .min_h(px(176.))
                        .child(div().relative().w(px(83.)).h(px(65.)).flex_none().child(
                            art("history-stack", 131., 113.).absolute().top(px(-24.)).left(px(-24.)),
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(text(tx("Le début d’une belle histoire."), 12., 650., c(theme::HEADING)).mb(px(7.)))
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .line_height(px(18.5))
                                        .text_color(hex(0xa0a895))
                                        .mb(px(10.))
                                        .child(tx("Vos diagnostics seront enregistrés ici."))
                                        .child(tx("Parfait pour comparer avant et après une mise à niveau.")),
                                )
                                .child(div().flex().child(
                                    text_button("first-report", tx("Créer mon premier bilan "), Some(("arrow-right", 15.)))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.navigate(Page::Diagnostics, window, cx)
                                        })),
                                )),
                        ),
                )
            } else {
                d.child(div().pb(px(12.)).px(px(22.)).flex().flex_col().children(reports.into_iter().map(|item| {
                    let report = item.clone();
                    div()
                        .id(SharedString::from(format!("recent-{}", item.id)))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .py(px(13.))
                        .border_t_1()
                        .border_color(c(theme::BORDER))
                        .cursor_pointer()
                        .child(
                            div()
                                .size(px(33.))
                                .flex_none()
                                .rounded(px(9.))
                                .bg(c((0xf1f5ea, 0x304129)))
                                .border_1()
                                .border_color(c((0xe8eee0, 0x405333)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon("activity", 18., "2", c((0x8da270, 0xa4c580)))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(one_line(item.label.clone(), 11., 550., c(theme::TEXT)))
                                .child(text(
                                    format!(
                                        "{} {} {}",
                                        intl::date(locale, item.created_at as i64, true),
                                        tx("·"),
                                        intl::duration(locale, item.duration_ms)
                                    ),
                                    9.,
                                    400.,
                                    hex(0xa0aa92),
                                )),
                        )
                        .child(score(item.summary.score, item.summary.status, false))
                        .child(div().ml(px(6.)).child(icon("chevron-right", 17., "2", hex(0xb2bda4))))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.report = Some(report.clone());
                            cx.notify();
                        }))
                })))
            }
        })
}

fn tip_card(cx: &mut Context<Computy>) -> impl IntoElement {
    let side = |label: &str, before: bool| {
        let (bg, border, color) = if before {
            (
                c((0xf5f7ee, 0x31472a)),
                c((0xdfe5cf, 0x475c34)),
                c((0xb1bb9a, 0xa3c17f)),
            )
        } else {
            (
                c((0xe7efda, 0x31472a)),
                c((0xd6e2c5, 0x475c34)),
                c((0x829766, 0xa3c17f)),
            )
        };
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .py(px(8.))
            .px(px(10.))
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded(px(7.))
            .child(spaced(tx(label), 8., 400., color, 0.5, 10.))
            .child(icon("memory-stick", 21., "2", color))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .child(text(if before { "16" } else { "32" }, 13., 500., color))
                    .child(text(tx("Gio"), 7., 500., color).ml(px(3.))),
            )
    };
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .min_w_0()
        .relative()
        .pt(px(22.))
        .px(px(23.))
        .pb(px(14.))
        .bg(c((0xf0f2e9, 0x253322)))
        .border_1()
        .border_color(c((0xe5e9db, 0x394a2e)))
        .rounded(px(14.))
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.))
                .mb(px(13.))
                .child(icon("sparkles", 14., "2", hex(0x97a17e)))
                .child(spaced(tx(" LE BON RÉFLEXE").trim().to_owned(), 9., 600., hex(0x97a17e), 1.3, 12.)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(spaced(tx("De nouvelles barrettes ?"), 17., 600., c(theme::HEADING), -0.2, 25.5))
                .child(spaced(tx("Gardez un point de repère."), 17., 600., c(theme::HEADING), -0.2, 25.5)),
        )
        .child(
            text(
                tx("Lancez un diagnostic avant le remplacement, puis un autre après. Computy met vos deux configurations côte à côte."),
                12.,
                400.,
                c((0x7a8b69, 0xb5c7a0)),
            )
            .line_height(px(21.6))
            .mt(px(10.))
            .max_w(px(295.)),
        )
        .child(
            div().mt(px(15.)).flex().child(
                text_button("discover-compare", tx("Découvrir la comparaison "), Some(("arrow-up-right", 16.)))
                    .on_click(cx.listener(|this, _, window, cx| this.navigate(Page::Compare, window, cx))),
            ),
        )
        .child(
            div()
                .mt(px(17.))
                .flex()
                .items_center()
                .justify_end()
                .gap(px(13.))
                .child(side("AVANT", true))
                .child(icon("arrow-right", 18., "2", hex(0xb1bb9a)))
                .child(side("APRÈS", false)),
        )
}

pub fn render(app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    let cards = cards(app);
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("BIENVENUE DANS COMPUTY"),
            "Votre matériel, en clair.",
            Some("Comprenez votre ordinateur. Prenez-en soin, au bon moment."),
            Some(
                button(
                    "new-diagnostic",
                    ButtonKind::Secondary,
                    false,
                    Some("plus"),
                    tx("Nouveau diagnostic"),
                    false,
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.navigate(Page::Diagnostics, window, cx)),
                )
                .into_any_element(),
            ),
        ))
        .child(
            div()
                .flex()
                .gap(px(20.))
                .mb(px(31.))
                .child(hero(app, cx))
                .child(health_card(app, cx)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .mb(px(16.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(spaced(
                            tx("À l’intérieur de votre PC ").trim().to_owned(),
                            16.,
                            650.,
                            c(theme::HEADING),
                            -0.4,
                            21.,
                        ))
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(4.))
                                .rounded(px(4.))
                                .bg(c((0xedf1e6, 0x303d29)))
                                .child(text(
                                    tx("3 composants"),
                                    10.,
                                    500.,
                                    c((0x8a947e, 0x9bb086)),
                                )),
                        ),
                )
                .child(div().flex().items_center().gap(px(6.)).map(|d| {
                    if app.loading {
                        d.child(components::spinner(13., hex(0x9aa48e))).child(text(
                            tx(" Lecture du matériel").trim().to_owned(),
                            10.,
                            400.,
                            hex(0x9aa48e),
                        ))
                    } else {
                        d.child(div().size(px(4.)).rounded_full().bg(hex(0x87a66e)))
                            .child(text(
                                tx(" Données système").trim().to_owned(),
                                10.,
                                400.,
                                hex(0x9aa48e),
                            ))
                    }
                })),
        )
        .child(
            div().flex().gap(px(18.)).mb(px(28.)).children(
                cards
                    .into_iter()
                    .map(|card| component_card(app, card, cx).into_any_element()),
            ),
        )
        .child(
            div()
                .flex()
                .gap(px(20.))
                .child(recent_panel(app, cx))
                .child(tip_card(cx)),
        )
        .child(
            div()
                .mt(px(24.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("shield-check", 15., "2", hex(0x7e8f6e)))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .gap(px(4.))
                        .child(text(
                            tx("Des résultats utiles, des limites claires. ")
                                .trim()
                                .to_owned(),
                            10.,
                            400.,
                            hex(0x7e8f6e),
                        ))
                        .child(text(
                            tx("Un diagnostic sans erreur ne garantit pas l’absence de panne."),
                            10.,
                            400.,
                            hex(0xb1b8a9),
                        )),
                )
                .child(
                    div()
                        .id("method")
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .cursor_pointer()
                        .child(text(
                            tx("Notre méthode ").trim().to_owned(),
                            10.,
                            400.,
                            hex(0x8c9b78),
                        ))
                        .child(icon("arrow-up-right", 13., "2", hex(0x8c9b78)))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.navigate(Page::About, window, cx)
                        })),
                ),
        )
        .into_any_element()
}
