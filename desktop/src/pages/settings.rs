//! Preferences, tray monitor and maintenance (former Settings.tsx,
//! MonitorSettings.tsx and MaintenanceSettings.tsx).
use crate::{
    app::{Computy, Page},
    components::{
        self, ButtonKind, NoticeTone, button, button_foreground, button_with, notice, spinner,
    },
    forms::{field, native_checkbox, option, select},
    i18n::{LANGUAGES, tp, tx},
    model::{MonitorSnapshot, Preferences},
    monitor_ui::metric_visual,
    platform,
    theme::{self, c, hex},
    ui::{icon, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use serde_json::json;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub enum Maintenance {
    Uninstall,
    Erase,
}

pub struct SettingsState {
    pub draft: Preferences,
    startup: bool,
    startup_busy: bool,
    snapshot: Option<MonitorSnapshot>,
    problem: bool,
    kind: String,
    pub action: Option<Maintenance>,
    confirmed: bool,
    busy: bool,
    _poll: Task<()>,
}

fn state(app: &mut Computy) -> &mut SettingsState {
    app.pages.settings.as_mut().expect("settings state")
}

impl SettingsState {
    pub fn new(app: &Computy, cx: &mut Context<Computy>) -> Self {
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok((backend, enabled)) = this.read_with(cx, |this, _| {
                    (
                        this.backend.clone(),
                        this.pages
                            .settings
                            .as_ref()
                            .is_some_and(|s| s.draft.tray_enabled),
                    )
                }) else {
                    break;
                };
                let fixture = backend.is_fixture();
                if enabled {
                    let result = cx
                        .background_executor()
                        .spawn(
                            async move { backend.typed::<MonitorSnapshot>("monitor", json!({})) },
                        )
                        .await;
                    let alive = this.update(cx, |this, cx| {
                        if let Some(s) = this.pages.settings.as_mut() {
                            match result {
                                Ok(snapshot) => {
                                    s.snapshot = Some(snapshot);
                                    s.problem = false;
                                }
                                Err(_) => {
                                    s.snapshot = None;
                                    s.problem = true;
                                }
                            }
                        }
                        cx.notify();
                    });
                    if alive.is_err() || fixture {
                        break;
                    }
                }
                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        });
        let (startup, kind) = match &app.backend {
            crate::backend::Backend::Fixture(_) => (
                app.backend.host::<bool>("startup_status").unwrap_or(false),
                app.backend
                    .host::<String>("maintenance_info")
                    .unwrap_or_else(|| "portable".into()),
            ),
            crate::backend::Backend::Live(_) => (
                platform::startup_status(),
                platform::installation()
                    .map(|(k, _)| k.to_owned())
                    .unwrap_or_else(|_| "portable".into()),
            ),
        };
        Self {
            draft: app.preferences.clone(),
            startup,
            startup_busy: false,
            snapshot: None,
            problem: false,
            kind,
            action: None,
            confirmed: false,
            busy: false,
            _poll: poll,
        }
    }
}

fn toggle_switch(id: &'static str, on: bool, busy: bool) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(36.))
        .h(px(21.))
        .flex_none()
        .p(px(2.))
        .rounded(px(20.))
        .border_1()
        .flex()
        .items_center()
        .map(|d| {
            if on {
                d.justify_end()
                    .bg(c((0x8eac6d, 0x8eac6d)))
                    .border_color(c((0x87a663, 0x87a663)))
            } else {
                d.justify_start()
                    .bg(c((0xe5eadf, 0x39492f)))
                    .border_color(c((0xdbe2d3, 0x425435)))
            }
        })
        .cursor_pointer()
        .child(if busy {
            spinner(13., hex(0xffffff))
        } else {
            div()
                .size(px(15.))
                .rounded_full()
                .bg(hex(0xffffff))
                .shadow(vec![BoxShadow {
                    color: theme::hexa(0x526c24, 0.13),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(3.),
                    spread_radius: px(0.),
                }])
                .into_any_element()
        })
}

fn setting_row(
    icon_name: &str,
    title: &str,
    body: &str,
    control: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(15.))
        .px(px(23.))
        .pb(px(25.))
        .child(
            div()
                .size(px(40.))
                .flex_none()
                .rounded(px(9.))
                .bg(c((0xf1f6e9, 0x30452a)))
                .border_1()
                .border_color(c((0xe4edd8, 0x425b32)))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(icon_name, 22., "2", hex(0x9cb582))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(text(tx(title), 13., 550., c(theme::TEXT)))
                .child(
                    text(tx(body), 12., 400., c((0x7a9066, 0xadc29a)))
                        .line_height(px(19.8))
                        .mt(px(7.)),
                ),
        )
        .child(control)
}

fn theme_option(
    app: &Computy,
    id: &'static str,
    label: &str,
    icon_name: &str,
    cx: &mut Context<Computy>,
) -> impl IntoElement {
    let selected = app
        .pages
        .settings
        .as_ref()
        .is_some_and(|s| s.draft.theme == id);
    let (bg, side, side_border, card, card_border) = match id {
        "dark" => (
            hex(0x202a24),
            hex(0x16221a),
            hex(0x364834),
            hex(0x2c3a2b),
            hex(0x3b4c35),
        ),
        _ => (
            hex(0xf7f9f4),
            hex(0xffffff),
            hex(0xe7ebde),
            hex(0xffffff),
            hex(0xe4e9d9),
        ),
    };
    let preview_card = |bg: Hsla, border: Hsla| {
        div()
            .w(px(45.))
            .h(px(36.))
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded(px(4.))
    };
    let preview = div()
        .relative()
        .h(px(95.))
        .border_1()
        .border_color(if id == "dark" {
            hex(0x344537)
        } else {
            c((0xe1e8d8, 0x78995d))
        })
        .rounded(px(5.))
        .overflow_hidden()
        .bg(bg)
        .pt(px(38.))
        .pr(px(10.))
        .pb(px(10.))
        .pl(px(40.))
        .flex()
        .gap(px(6.))
        .when(id == "system", |d| {
            d.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(relative(0.5))
                    .bg(hex(0x202a24)),
            )
        })
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(29.))
                .bg(side)
                .border_r_1()
                .border_color(side_border),
        )
        .map(|d| match id {
            "system" => d
                .child(preview_card(hex(0xffffff), hex(0xe4e9d9)))
                .child(preview_card(hex(0x34482d), hex(0x496039)))
                .child(preview_card(hex(0x34482d), hex(0x496039))),
            _ => d
                .child(preview_card(card, card_border))
                .child(preview_card(card, card_border))
                .child(preview_card(card, card_border)),
        });
    let color = hex(0x8ca174);
    div()
        .id(SharedString::from(format!("theme-{id}")))
        .w(px(220.))
        .flex_none()
        .p(px(10.))
        .rounded(px(10.))
        .border_1()
        .overflow_hidden()
        .cursor_pointer()
        .map(|d| {
            if selected {
                d.bg(c((0xf2f7e9, 0x34492b)))
                    .border_color(c((0xa8be8a, 0x789957)))
            } else {
                d.border_color(c(theme::BORDER))
            }
        })
        .flex()
        .flex_col()
        .when(!selected && id != "system", |d| d.pt(px(32.)))
        .child(preview)
        .child(
            div()
                .mt(px(11.))
                .px(px(3.))
                .pb(px(2.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon(icon_name, 17., "2", color))
                .child(text(tx(label), 12., 400., color))
                .child(div().flex_1())
                .when(selected, |d| d.child(icon("check", 15., "2", color))),
        )
        .when(id == "system", |d| {
            d.child(div().pt(px(7.)).px(px(3.)).pb(px(2.)).child(text(
                tx("S’adapte automatiquement"),
                10.,
                400.,
                hex(0x96ac7d),
            )))
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            state(this).draft.theme = id.into();
            cx.notify();
        }))
}

fn monitor_settings(app: &Computy, cx: &mut Context<Computy>) -> impl IntoElement {
    let s = app.pages.settings.as_ref().unwrap();
    let draft = s.draft.clone();
    let snapshot = s.snapshot.clone();
    let note = |content: String| {
        text(content, 12., 400., c(theme::TEXT))
            .line_height(px(20.4))
            .opacity(0.82)
    };
    let style_option = |id: &'static str, label: &str, cx: &mut Context<Computy>| {
        let selected = draft.tray_style == id;
        div()
            .id(SharedString::from(format!("tray-style-{id}")))
            .flex_1()
            .min_w_0()
            .h(px(165.))
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .pt(px(23.))
            .px(px(12.))
            .pb(px(16.))
            .rounded(px(14.))
            .border_1()
            .cursor_pointer()
            .map(|d| {
                if selected {
                    d.bg(c((0xeaf1e7, 0x2a3e24))).border_color(c(theme::GREEN))
                } else {
                    d.bg(c((0xf7f9f6, 0x253324))).border_color(c(theme::BORDER))
                }
            })
            .child(
                div()
                    .absolute()
                    .top(px(16.))
                    .right(px(12.))
                    .size(px(13.))
                    .rounded_full()
                    .border_1()
                    .border_color(if selected {
                        c(theme::GREEN)
                    } else {
                        hex(0x767676)
                    })
                    .bg(hex(0xffffff))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(selected, |d| {
                        d.child(div().size(px(7.)).rounded_full().bg(c(theme::GREEN)))
                    }),
            )
            .child(metric_visual(Some(42.), "memory", id, 65., 65.))
            .child(text(tx(label), 12., 700., c(theme::TEXT)))
            .when(id == "icon_text", |d| {
                d.child(
                    div()
                        .py(px(3.))
                        .px(px(8.))
                        .rounded(px(20.))
                        .bg(c(theme::PANEL))
                        .child(text(tx("Par défaut"), 10., 400., hex(0x6d7c70))),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                state(this).draft.tray_style = id.into();
                cx.notify();
            }))
    };
    let metric_option =
        |id: &'static str, name: &str, value: Option<f64>, cx: &mut Context<Computy>| {
            let selected = draft.tray_components.iter().any(|c| c == id);
            div()
                .id(SharedString::from(format!("metric-{id}")))
                .flex_1()
                .min_w_0()
                .relative()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(9.))
                .py(px(18.))
                .px(px(12.))
                .rounded(px(14.))
                .border_1()
                .cursor_pointer()
                .map(|d| {
                    if selected {
                        d.bg(c((0xeaf1e7, 0x2a3e24)))
                            .border_color(c((0x87a963, 0x6d8954)))
                    } else {
                        d.bg(c((0xf7f9f6, 0x253324))).border_color(c(theme::BORDER))
                    }
                })
                .child(
                    div()
                        .absolute()
                        .top(px(13.))
                        .right(px(9.))
                        .child(native_checkbox(selected, false, hex(0x5f803c))),
                )
                .child(metric_visual(value, id, &draft.tray_style, 86., 80.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(icon(
                            crate::monitor_ui::metric_icon(id),
                            15.,
                            "2",
                            c(theme::TEXT),
                        ))
                        .child(text(tx(name), 13., 700., c(theme::TEXT))),
                )
                .child(note(value.map_or_else(
                    || tx("Indisponible"),
                    |v| tp("Utilisation : {{value}} %", &[("value", v.round().into())]),
                )))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let d = &mut state(this).draft;
                    if let Some(i) = d.tray_components.iter().position(|c| c == id) {
                        d.tray_components.remove(i);
                    } else {
                        d.tray_components.push(id.into());
                    }
                    cx.notify();
                }))
        };
    let mut disks: Vec<(String, String, Option<f64>)> = snapshot
        .as_ref()
        .map(|s| {
            s.disks
                .iter()
                .map(|d| (d.id.clone(), d.name.clone(), d.used_percent))
                .collect()
        })
        .unwrap_or_default();
    for id in &draft.tray_disks {
        if !disks.iter().any(|d| &d.0 == id) {
            disks.push((id.clone(), String::new(), None));
        }
    }
    let enabled = draft.tray_enabled;
    panel()
        .mb(px(20.))
        .flex()
        .flex_col()
        .child(components::panel_header("Moniteur dans la barre système", Some(tx("En direct dans votre barre système.")), None))
        .child(setting_row(
            "activity",
            "Afficher les indicateurs en temps réel",
            "Activé par défaut : RAM, CPU et GPU. Les changements s’appliquent après l’enregistrement des préférences.",
            toggle_switch("tray-enabled", enabled, false).on_click(cx.listener(|this, _, _, cx| {
                let d = &mut state(this).draft;
                d.tray_enabled = !d.tray_enabled;
                cx.notify();
            })),
        ))
        .when(enabled, |d| {
            d.child(setting_row(
                "app-window",
                "Continuer en arrière-plan à la fermeture",
                "Fermer la fenêtre laisse Computy et ses indicateurs actifs. Pour quitter, utilisez le menu de l’indicateur.",
                toggle_switch("keep-running", draft.keep_running_on_close, false).on_click(cx.listener(|this, _, _, cx| {
                    let d = &mut state(this).draft;
                    d.keep_running_on_close = !d.keep_running_on_close;
                    cx.notify();
                })),
            ))
        })
        .when(enabled, |d| {
            d.child(
                div()
                    .px(px(28.))
                    .pb(px(25.))
                    .flex()
                    .flex_col()
                    .child(text(tx("Style de la barre système"), 14., 650., c(theme::HEADING)).px(px(2.)))
                    .child(
                        div()
                            .mt(px(14.))
                            .flex()
                            .gap(px(12.))
                            .child(style_option("icon_text", "Icône + pourcentage", cx))
                            .child(style_option("ring", "Anneau", cx))
                            .child(style_option("fill", "Icône remplie", cx)),
                    )
                    .child(
                        div()
                            .mt(px(12.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(7.))
                            .opacity(0.82)
                            .text_size(px(12.))
                            .line_height(px(20.4))
                            .text_color(c(theme::TEXT))
                            .child(div().ml(px(8.)).size(px(8.)).rounded_full().bg(hex(0x429feb)))
                            .child(format!("{} < 65 %", tx("Bleu")))
                            .child(div().ml(px(8.)).size(px(8.)).rounded_full().bg(hex(0xdca842)))
                            .child(format!("{} 65–89 %", tx("Jaune")))
                            .child(div().ml(px(8.)).size(px(8.)).rounded_full().bg(hex(0xe96565)))
                            .child(format!("{} ≥ 90 %", tx("Rouge"))),
                    )
                    .when(snapshot.is_none() && !s.problem, |d| {
                        d.child(div().mt(px(12.)).flex().items_center().gap(px(6.)).child(spinner(16., c(theme::MUTED))).child(text(tx("Lecture des mesures en direct…"), 12., 400., c(theme::MUTED))))
                    })
                    .when(s.problem, |d| {
                        d.child(notice(NoticeTone::Warn, vec![tx("Les mesures en direct sont indisponibles. Computy réessaie automatiquement.").into_any_element()]).mt(px(12.)))
                    })
                    .child(
                        div()
                            .mt(px(20.))
                            .flex()
                            .gap(px(14.))
                            .child(metric_option("memory", "Mémoire vive", snapshot.as_ref().and_then(|s| s.memory_percent), cx))
                            .child(metric_option("cpu", "Processeur", snapshot.as_ref().and_then(|s| s.cpu_percent), cx))
                            .child(metric_option("gpu", "Processeur graphique", snapshot.as_ref().and_then(|s| s.gpu_percent), cx)),
                    )
                    .child(note(tx("CPU et RAM : actualisation toutes les 2 secondes. GPU : jusqu’à 6 secondes selon le pilote. « — » signifie qu’aucune mesure n’est disponible, pas 0 %.")).mt(px(21.)))
                    .child(div().flex().child(button("all-processes", ButtonKind::Secondary, true, None, tx("Voir tous les processus"), false).on_click(cx.listener(|this, _, window, cx| this.navigate(Page::Processes, window, cx)))))
                    .child(note(tx("Le GPU dépend des compteurs exposés par votre pilote : AMD/NVIDIA sous Linux, compteurs Windows et certains pilotes IOAccelerator sous macOS. Les GPU non pris en charge restent indiqués comme indisponibles.")))
                    .child(
                        div()
                            .mt(px(24.))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(icon("hard-drive", 17., "2", c(theme::HEADING)))
                            .child(spaced(tx("Disques à afficher"), 15., 650., c(theme::HEADING), -0.2, 20.)),
                    )
                    .child(note(tx("Un indicateur par volume sélectionné : l’anneau montre l’espace occupé ; le menu affiche aussi les débits de lecture et d’écriture. Ce ne sont pas des notes de santé.")).mt(px(6.)))
                    .child(
                        div().mt(px(15.)).flex().flex_col().gap(px(8.)).children(disks.into_iter().map(|(id, name, used)| {
                            let checked = draft.tray_disks.contains(&id);
                            let disabled = !checked && draft.tray_disks.len() >= 16;
                            let toggle = id.clone();
                            div()
                                .id(SharedString::from(format!("disk-{id}")))
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .py(px(13.))
                                .px(px(15.))
                                .border_1()
                                .border_color(c(theme::BORDER))
                                .rounded(px(10.))
                                .cursor_pointer()
                                .child(native_checkbox(checked, disabled, hex(0x5f803c)))
                                .child(icon("hard-drive", 18., "2", c(theme::TEXT)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(text(id.clone(), 12., 700., c(theme::TEXT)))
                                        .child(text(name, 10., 400., c(theme::TEXT)).opacity(0.82)),
                                )
                                .child(text(
                                    used.map_or_else(|| tx("Indisponible"), |v| tp("Espace occupé : {{value}} %", &[("value", v.round().into())])),
                                    12.,
                                    400.,
                                    c(theme::TEXT),
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if disabled {
                                        return;
                                    }
                                    let d = &mut state(this).draft;
                                    if let Some(i) = d.tray_disks.iter().position(|v| *v == toggle) {
                                        d.tray_disks.remove(i);
                                    } else {
                                        d.tray_disks.push(toggle.clone());
                                    }
                                    cx.notify();
                                }))
                                .into_any_element()
                        })),
                    )
                    .child(
                        notice(NoticeTone::Info, vec![tx("Fermer la fenêtre conserve les indicateurs en arrière-plan. Le menu de chaque indicateur permet de rouvrir Computy, d’accéder aux préférences ou de quitter complètement.").into_any_element()])
                            .mt(px(18.)),
                    )
                    .child(note(tx("Linux : le bureau doit prendre en charge les indicateurs AppIndicator/StatusNotifier. Sous GNOME, une extension de prise en charge peut être nécessaire. Windows peut ranger les icônes dans le menu des icônes masquées.")).mt(px(14.))),
            )
        })
}

pub fn render(app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.settings.is_none() {
        let state = SettingsState::new(app, cx);
        app.pages.settings = Some(state);
    }
    let s = app.pages.settings.as_ref().unwrap();
    let draft = s.draft.clone();
    let (startup, startup_busy, kind) = (s.startup, s.startup_busy, s.kind.clone());
    let running = app.job.as_ref().is_some_and(|j| j.running);
    let locale = crate::i18n::locale();
    let mib = |v: u64| format!("{}{}", crate::intl::plain(locale, v as f64), tx(" Mio"));
    let edit = |key: &'static str| {
        move |this: &mut Computy, value: String, _: &mut Window, _: &mut Context<Computy>| {
            let v: u64 = value.parse().unwrap_or(0);
            let d = &mut state(this).draft;
            match key {
                "memory_mib" => d.memory_mib = v,
                "passes" => d.passes = v,
                "disk_mib" => d.disk_mib = v,
                _ => d.cpu_seconds = v,
            }
        }
    };
    let mut languages = vec![option("system", tx("Suivre le système"))];
    languages.extend(LANGUAGES.iter().map(|l| option(l.code, l.name)));
    let secondary = button_foreground(ButtonKind::Secondary);
    let primary = button_foreground(ButtonKind::Primary);
    div()
        .max_w(px(1000.))
        .flex()
        .flex_col()
        .child(components::section_title(Some("À VOTRE FAÇON"), "Un espace qui vous ressemble.", Some("Personnalisez l’apparence et les paramètres de vos prochains diagnostics."), None))
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(components::panel_header("Langue de l’interface", Some(tx("Les traductions sont incluses dans l’application. Aucun service de traduction n’est appelé.")), None))
                .child(
                    div()
                        .px(px(23.))
                        .pb(px(25.))
                        .flex()
                        .flex_col()
                        .child(text(tx("Langue préférée"), 12., 400., hex(0x7c9168)))
                        .child(
                            div().mt(px(10.)).w(px(260.)).child(
                                select("interface-language", &draft.language, languages, app, cx, |this, value, _, _| state(this).draft.language = value)
                                    .h(px(43.))
                                    .px(px(13.))
                                    .border_color(c((0xe8ece7, 0x425435))),
                            ),
                        )
                        .child(text(tx("25 langues · dates et nombres localisés · langues de droite à gauche prises en charge"), 11., 400., hex(0x8ba375)).line_height(px(18.15)).mt(px(12.))),
                ),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(components::panel_header("Apparence", Some(tx("Le même espace, selon votre préférence.")), None))
                .child(
                    div()
                        .flex()
                        .gap(px(18.))
                        .px(px(23.))
                        .pb(px(25.))
                        .child(theme_option(app, "system", "Système", "laptop", cx))
                        .child(theme_option(app, "light", "Clair", "sun", cx))
                        .child(theme_option(app, "dark", "Sombre", "moon", cx)),
                ),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(components::panel_header("Votre profil « L’essentiel »", Some(tx("Ces valeurs seront proposées lors d’un nouveau diagnostic.")), None))
                .child(
                    div()
                        .px(px(23.))
                        .pb(px(23.))
                        .flex()
                        .flex_col()
                        .gap(px(20.))
                        .child(
                            div()
                                .flex()
                                .gap(px(20.))
                                .child(field("Mémoire à allouer", select("pref-memory", &draft.memory_mib.to_string(), [16, 32, 64, 128, 256, 512, 1024, 2048].map(|v| option(v, mib(v))).into_iter().collect(), app, cx, edit("memory_mib"))))
                                .child(field("Passes mémoire", select("pref-passes", &draft.passes.to_string(), [1, 2, 3, 4, 6, 8].map(|v| option(v, crate::i18n::tn(v as f64))).into_iter().collect(), app, cx, edit("passes")))),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(20.))
                                .child(field("Fichier temporaire", select("pref-disk", &draft.disk_mib.to_string(), [16, 32, 64, 128, 256, 512].map(|v| option(v, mib(v))).into_iter().collect(), app, cx, edit("disk_mib"))))
                                .child(field(
                                    "Calculs processeur",
                                    select(
                                        "pref-cpu",
                                        &draft.cpu_seconds.to_string(),
                                        [1, 2, 5, 10, 15, 30, 60].map(|v| option(v, format!("{}{}", crate::i18n::tn(v as f64), tx(" secondes")))).into_iter().collect(),
                                        app,
                                        cx,
                                        edit("cpu_seconds"),
                                    ),
                                )),
                        ),
                )
                .child(div().px(px(23.)).pb(px(23.)).child(notice(NoticeTone::Info, vec![tx("Le moteur garde une réserve de mémoire et d’espace disque. Si l’allocation est trop élevée pour les ressources disponibles, le contrôle est refusé plutôt que de saturer votre PC.").into_any_element()]))),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(components::panel_header("Démarrage automatique", None, None))
                .child(setting_row(
                    "laptop",
                    "Afficher les indicateurs à la connexion",
                    "Démarre le moniteur en arrière-plan, sans ouvrir la fenêtre principale.",
                    toggle_switch("startup", startup, startup_busy).on_click(cx.listener(move |this, _, _, cx| {
                        if startup_busy {
                            return;
                        }
                        let Some(service) = this.backend.service().cloned() else { return };
                        state(this).startup_busy = true;
                        let task = cx.background_executor().spawn(async move { platform::set_startup(&service, !startup) });
                        cx.spawn(async move |this, cx| {
                            let result = task.await;
                            let _ = this.update(cx, |this, cx| {
                                state(this).startup_busy = false;
                                match result {
                                    Ok(()) => {
                                        state(this).startup = !startup;
                                        this.notify(tx(if !startup { "Le moniteur démarrera à la connexion." } else { "Démarrage automatique désactivé." }), false, cx);
                                    }
                                    Err(error) => this.notify(error, true, cx),
                                }
                            });
                        })
                        .detach();
                        cx.notify();
                    })),
                )),
        )
        .child(monitor_settings(app, cx))
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(12.))
                .mb(px(24.))
                .child(
                    button_with("defaults", ButtonKind::Secondary, false, Some(icon("rotate-ccw", 16., "2", secondary).into_any_element()), tx("Valeurs par défaut"), false).on_click(cx.listener(|this, _, _, cx| {
                        state(this).draft = Preferences::default();
                        cx.notify();
                    })),
                )
                .child(
                    button_with("save-preferences", ButtonKind::Primary, false, Some(icon("save", 16., "2", primary).into_any_element()), tx("Enregistrer les préférences"), false).on_click(cx.listener(|this, _, window, cx| {
                        let draft = state(this).draft.clone();
                        this.save_preferences(draft, window, cx);
                    })),
                ),
        )
        .child(
            panel()
                .flex()
                .flex_col()
                .child(components::panel_header("Installation et données", None, None))
                .child(setting_row(
                    "package-minus",
                    "Désinstaller Computy",
                    if kind == "portable" { "Version portable : supprimez le fichier ou le dossier de l’application." } else { "Le système demandera votre autorisation. Les données sont conservées." },
                    button_with("uninstall", ButtonKind::Secondary, true, Some(icon("package-minus", 16., "2", secondary).into_any_element()), tx("Désinstaller Computy"), kind == "portable" || running).on_click(cx.listener(move |this, _, _, cx| {
                        if kind != "portable" && !running {
                            state(this).action = Some(Maintenance::Uninstall);
                            cx.notify();
                        }
                    })),
                ))
                .child(setting_row(
                    "trash-2",
                    "Supprimer toutes les données",
                    "L’historique, les préférences et les imports en attente seront supprimés. Les exports personnels sont conservés.",
                    button_with("erase", ButtonKind::Danger, true, Some(icon("trash-2", 16., "2", hex(0xffffff)).into_any_element()), tx("Supprimer toutes les données"), running).on_click(cx.listener(move |this, _, _, cx| {
                        if !running {
                            state(this).action = Some(Maintenance::Erase);
                            cx.notify();
                        }
                    })),
                )),
        )
        .into_any_element()
}

pub fn modal(
    app: &mut Computy,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> Option<AnyElement> {
    let s = app.pages.settings.as_ref()?;
    let action = s.action?;
    let (confirmed, busy) = (s.confirmed, s.busy);
    let title = if action == Maintenance::Erase {
        "Supprimer toutes les données"
    } else {
        "Désinstaller Computy"
    };
    let explain = "L’historique, les préférences et les imports en attente seront supprimés. Les exports personnels sont conservés.";
    let close = std::rc::Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
        if !busy {
            let s = state(this);
            s.action = None;
            s.confirmed = false;
            cx.notify();
        }
    }));
    let body = div()
        .p(px(24.))
        .flex()
        .flex_col()
        .child(
            text(
                tx(if action == Maintenance::Erase {
                    explain
                } else {
                    "Le système demandera votre autorisation. Les données sont conservées."
                }),
                12.,
                400.,
                c(theme::MUTED),
            )
            .line_height(px(20.4))
            .mb(px(20.)),
        )
        .when(action == Maintenance::Erase, |d| {
            d.child(
                text(
                    tx("Cette action est irréversible."),
                    12.,
                    700.,
                    c(theme::TEXT),
                )
                .mb(px(20.)),
            )
        })
        .child(
            div()
                .id("maintenance-confirm")
                .flex()
                .items_center()
                .gap(px(5.))
                .cursor_pointer()
                .child(native_checkbox(
                    confirmed,
                    busy,
                    hex(crate::forms::BROWSER_ACCENT),
                ))
                .child(text(tx(title), 12., 400., c(theme::TEXT)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !busy {
                        let s = state(this);
                        s.confirmed = !s.confirmed;
                        cx.notify();
                    }
                })),
        )
        .child(
            div()
                .mt(px(20.))
                .flex()
                .justify_end()
                .gap(px(12.))
                .child(
                    button(
                        "maintenance-cancel",
                        ButtonKind::Secondary,
                        false,
                        None,
                        tx("Annuler"),
                        busy,
                    )
                    .on_click({
                        let close = close.clone();
                        move |e, w, cx| close(e, w, cx)
                    }),
                )
                .child(
                    button_with(
                        "maintenance-run",
                        ButtonKind::Danger,
                        false,
                        Some(if busy {
                            spinner(16., hex(0xffffff))
                        } else {
                            icon("trash-2", 16., "2", hex(0xffffff)).into_any_element()
                        }),
                        tx(title),
                        !confirmed || busy,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !confirmed || busy {
                            return;
                        }
                        let Some(service) = this.backend.service().cloned() else {
                            return;
                        };
                        state(this).busy = true;
                        let task = cx.background_executor().spawn(async move {
                            match action {
                                Maintenance::Erase => {
                                    if service.active_job().is_some_and(|j| j.running) {
                                        return Err(
                                            "Arrêtez le diagnostic avant de continuer.".to_owned()
                                        );
                                    }
                                    service.erase_data(true)
                                }
                                Maintenance::Uninstall => platform::uninstall(&service),
                            }
                        });
                        cx.spawn(async move |this, cx| {
                            let result = task.await;
                            let _ = this.update(cx, |this, cx| {
                                let s = state(this);
                                s.busy = false;
                                match result {
                                    Ok(()) => {
                                        s.action = None;
                                        s.confirmed = false;
                                        if action == Maintenance::Uninstall {
                                            crate::quit(cx);
                                        } else {
                                            this.reload_history(cx);
                                            this.preferences = Preferences::default();
                                        }
                                    }
                                    Err(error) => this.notify(error, true, cx),
                                }
                            });
                        })
                        .detach();
                        cx.notify();
                    })),
                ),
        )
        .into_any_element();
    Some(components::modal(
        "maintenance",
        tx(title),
        false,
        body,
        close,
        window,
    ))
}
