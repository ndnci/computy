//! Installed applications (former pages/Applications.tsx).
use crate::{
    app::Computy,
    components::{self, ButtonKind, button_foreground, button_with, spinner},
    forms::{option, select},
    i18n::{tp, tx},
    model::{Application, ApplicationCatalog},
    theme::{self, c, hex},
    ui::{icon, one_line, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::{Input, InputEvent, InputState};
use serde_json::{Value, json};

pub struct ApplicationsState {
    pub apps: Vec<Application>,
    pub warnings: Vec<String>,
    pub loading: bool,
    pub busy: String,
    pub query: Entity<InputState>,
    pub manager: String,
    pub show_packages: bool,
    pub visible: usize,
    pub selected: Option<Application>,
    pub removal: bool,
    pub details: String,
    pub remove_data: bool,
    _subscription: Subscription,
}

fn state(app: &mut Computy) -> &mut ApplicationsState {
    app.pages.applications.as_mut().expect("applications state")
}

fn load(app: &mut Computy, cx: &mut Context<Computy>) {
    state(app).loading = true;
    app.call(
        "applications",
        json!({}),
        cx,
        |this, result: Result<ApplicationCatalog, String>, cx| {
            state(this).loading = false;
            match result {
                Ok(catalog) => {
                    let s = state(this);
                    s.apps = catalog.apps;
                    s.warnings = catalog.warnings;
                }
                Err(error) => this.notify(error, true, cx),
            }
        },
    );
}

impl ApplicationsState {
    pub fn new(window: &mut Window, cx: &mut Context<Computy>) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder(tx("Rechercher une application…")));
        let subscription = cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(s) = this.pages.applications.as_mut() {
                    s.visible = 60;
                }
                cx.notify();
            }
        });
        Self {
            apps: vec![],
            warnings: vec![],
            loading: true,
            busy: String::new(),
            query,
            manager: "all".into(),
            show_packages: false,
            visible: 60,
            selected: None,
            removal: false,
            details: String::new(),
            remove_data: false,
            _subscription: subscription,
        }
    }
}

fn utility_check(id: &'static str, checked: bool, disabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(7.))
        .when(!disabled, |d| d.cursor_pointer())
        .child(crate::forms::native_checkbox(
            checked,
            disabled,
            hex(crate::forms::BROWSER_ACCENT),
        ))
}

fn card(app: &Application, cx: &mut Context<Computy>) -> impl IntoElement {
    let selected = app.clone();
    let subtitle = app
        .version
        .clone()
        .filter(|v| !v.is_empty())
        .or_else(|| app.package.clone())
        .unwrap_or_default();
    div()
        .id(SharedString::from(format!("app-{}", app.id)))
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(13.))
        .p(px(18.))
        .min_h(px(104.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(12.))
        .cursor_pointer()
        .hover(|s| s.border_color(c((0xcad9bc, 0x4d6340))))
        .child(
            div()
                .size(px(48.))
                .flex_none()
                .rounded(px(13.))
                .bg(c((0xf6f8f6, 0x17221b)))
                .border_1()
                .border_color(c(theme::BORDER))
                .flex()
                .items_center()
                .justify_center()
                .child(icon("app-window", 24., "2", c(theme::GREEN))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(spaced(app.name.clone(), 12., 650., c(theme::HEADING), -0.2, 16.).nowrap())
                .when(!subtitle.is_empty(), |d| {
                    d.child(
                        one_line(subtitle, 10., 400., hex(0x6d7c70))
                            .line_height(px(16.5))
                            .mt(px(5.)),
                    )
                })
                .child(
                    div().mt(px(9.)).flex().child(
                        div()
                            .py(px(2.))
                            .px(px(6.))
                            .border_1()
                            .border_color(c(theme::BORDER))
                            .rounded(px(5.))
                            .child(text(app.manager.clone(), 9., 400., hex(0x6d7c70))),
                    ),
                ),
        )
        .child(icon("external-link", 15., "2", hex(0x6d7c70)))
        .on_click(cx.listener(move |this, _, _, cx| {
            let s = state(this);
            s.selected = Some(selected.clone());
            s.removal = false;
            cx.notify();
        }))
}

pub fn render(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.applications.is_none() {
        let state = ApplicationsState::new(window, cx);
        app.pages.applications = Some(state);
        load(app, cx);
    }
    let s = app.pages.applications.as_ref().unwrap();
    let query = s.query.read(cx).value().to_lowercase();
    let filtered: Vec<Application> = s
        .apps
        .iter()
        .filter(|a| {
            (s.show_packages || !a.system)
                && (s.manager == "all" || a.manager == s.manager)
                && format!(
                    "{} {} {}",
                    a.name,
                    a.package.as_deref().unwrap_or(""),
                    a.location.as_deref().unwrap_or("")
                )
                .to_lowercase()
                .contains(&query)
        })
        .cloned()
        .collect();
    let mut managers: Vec<String> = s.apps.iter().map(|a| a.manager.clone()).collect();
    managers.sort();
    managers.dedup();
    let mut options = vec![option("all", tx("Tous les gestionnaires"))];
    options.extend(managers.into_iter().map(|m| option(m.clone(), m)));
    let loading = s.loading;
    let visible = s.visible.min(filtered.len());
    let cards: Vec<AnyElement> = filtered[..visible]
        .iter()
        .map(|a| card(a, cx).into_any_element())
        .collect();
    let mut rows = vec![];
    let mut iter = cards.into_iter().peekable();
    while iter.peek().is_some() {
        let row: Vec<AnyElement> = iter.by_ref().take(4).collect();
        let missing = 4 - row.len();
        rows.push(
            div()
                .flex()
                .gap(px(12.))
                .children(
                    row.into_iter()
                        .map(|c| div().flex_1().min_w_0().flex().child(c)),
                )
                .children((0..missing).map(|_| div().flex_1()))
                .into_any_element(),
        );
    }
    let secondary = button_foreground(ButtonKind::Secondary);
    let query_entity = s.query.clone();
    let manager = s.manager.clone();
    let show_packages = s.show_packages;
    let warnings = s.warnings.clone();
    let more = visible < filtered.len();
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("LOGICIELS"),
            "Vos applications, au même endroit.",
            Some("Retrouvez une installation, ouvrez une application ou utilisez son gestionnaire pour la désinstaller."),
            None,
        ))
        .child(
            panel()
                .mt(px(24.))
                .p(px(26.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex_1()
                                .h(px(38.))
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .px(px(12.))
                                .border_1()
                                .border_color(c(theme::BORDER))
                                .rounded(px(9.))
                                .text_size(px(13.))
                                .child(icon("search", 17., "2", hex(0x6d7c70)))
                                .child(div().flex_1().px(px(12.)).child(Input::new(&query_entity).appearance(false))),
                        )
                        .child(
                            div().w(px(168.)).child(
                                select("app-manager", &manager, options, app, cx, |this, value, _, _| {
                                    let s = state(this);
                                    s.manager = value;
                                    s.visible = 60;
                                })
                                .rounded(px(9.))
                                .border_color(c((0xe8ece7, 0x425435))),
                            ),
                        )
                        .child(
                            button_with(
                                "apps-refresh",
                                ButtonKind::Secondary,
                                true,
                                Some(if loading { spinner(15., secondary) } else { icon("refresh-cw", 15., "2", secondary).into_any_element() }),
                                tx("Actualiser"),
                                loading,
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !loading {
                                    load(this, cx);
                                }
                            })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(14.))
                        .mt(px(22.))
                        .h(px(55.))
                        .child(
                            utility_check("show-packages", show_packages, false)
                                .child(text(tx("Afficher aussi les paquets système"), 12., 600., c(theme::TEXT)).line_height(px(19.2)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let s = state(this);
                                    s.show_packages = !s.show_packages;
                                    s.visible = 60;
                                    cx.notify();
                                })),
                        )
                        .child(text(tp("{{count}} applications", &[("count", filtered.len().into())]), 11., 400., hex(0x6d7c70))),
                )
                .children(warnings.into_iter().map(|w| text(w, 12., 400., c(theme::WARN)).mb(px(12.)).into_any_element()))
                .child(if loading {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .py(px(40.))
                        .child(spinner(18., c(theme::MUTED)))
                        .child(text(tx("Inventaire en cours…"), 12., 400., c(theme::MUTED)))
                        .into_any_element()
                } else if filtered.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .py(px(40.))
                        .child(icon("package", 32., "2", c(theme::MUTED)))
                        .child(text(tx("Aucune application trouvée."), 12., 400., c(theme::MUTED)))
                        .into_any_element()
                } else {
                    div().flex().flex_col().gap(px(12.)).children(rows).into_any_element()
                })
                .when(more, |d| {
                    d.child(
                        div().mt(px(18.)).flex().child(
                            components::button("apps-more", ButtonKind::Secondary, false, None, tx("Voir plus"), false).on_click(cx.listener(|this, _, _, cx| {
                                state(this).visible += 60;
                                cx.notify();
                            })),
                        ),
                    )
                }),
        )
        .into_any_element()
}

fn act(
    app: &mut Computy,
    application: Application,
    action: &'static str,
    cx: &mut Context<Computy>,
) {
    state(app).busy = application.id.clone();
    app.call(
        "application_action",
        json!({"id": application.id, "action": action}),
        cx,
        |this, result: Result<Value, String>, cx| {
            state(this).busy.clear();
            match result {
                Ok(_) => this.notify(tx("Action terminée."), false, cx),
                Err(error) => this.notify(error, true, cx),
            }
        },
    );
}

pub fn modal(
    app: &mut Computy,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> Option<AnyElement> {
    let s = app.pages.applications.as_ref()?;
    let selected = s.selected.clone()?;
    let busy = !s.busy.is_empty();
    let primary = button_foreground(ButtonKind::Primary);
    let secondary = button_foreground(ButtonKind::Secondary);
    let close = std::rc::Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
        if !busy {
            let s = state(this);
            s.selected = None;
            s.removal = false;
            cx.notify();
        }
    }));
    let fact = |label: &str, value: String| {
        div()
            .flex()
            .gap(px(12.))
            .py(px(8.))
            .border_b_1()
            .border_color(c(theme::BORDER))
            .child(
                div()
                    .w(px(110.))
                    .flex_none()
                    .child(text(tx(label), 12., 600., c(theme::MUTED))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(text(value, 12., 400., c(theme::TEXT))),
            )
    };
    let launchable = selected.launcher.is_some()
        || selected.launch_target.is_some()
        || ["snap", "flatpak"].contains(&selected.manager.as_str());
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(14.))
                .child(
                    div()
                        .size(px(56.))
                        .rounded(px(13.))
                        .bg(c((0xf6f8f6, 0x17221b)))
                        .border_1()
                        .border_color(c(theme::BORDER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon("app-window", 32., "2", c(theme::GREEN))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(spaced(selected.name.clone(), 17., 650., c(theme::HEADING), -0.4, 22.))
                        .child(text(format!("{} · {}", selected.version.clone().unwrap_or_default(), selected.manager), 12., 400., c(theme::MUTED))),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(fact("Identifiant", selected.package.clone().filter(|p| !p.is_empty()).unwrap_or_else(|| selected.id.clone())))
                .child(fact("Installation", selected.location.clone().unwrap_or_else(|| tx("Inconnue"))))
                .child(fact("Lanceur", selected.launcher.clone().or(selected.launch_target.clone()).unwrap_or_else(|| tx("Indisponible")))),
        )
        .child(if s.removal {
            let remove_data = s.remove_data;
            let can_remove = selected.can_remove_data;
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(text(tx("Vérifier la désinstallation"), 14., 650., c(theme::HEADING)))
                .child(
                    div()
                        .p(px(12.))
                        .max_h(px(220.))
                        .rounded(px(9.))
                        .bg(c((0xf6f8f6, 0x17221b)))
                        .border_1()
                        .border_color(c(theme::BORDER))
                        .text_size(px(11.))
                        .font_family("monospace")
                        .child(s.details.clone()),
                )
                .child(text(tx("Le gestionnaire peut retirer des dépendances. Vérifiez la liste avant de confirmer. Une autorisation du système peut être demandée."), 12., 400., c(theme::MUTED)))
                .child(
                    utility_check("remove-data", remove_data, !can_remove || busy)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(text(tx("Supprimer aussi les données gérées par le paquet"), 12., 600., c(theme::TEXT)))
                                .child(text(
                                    tx(if can_remove {
                                        "Les documents enregistrés ailleurs restent conservés."
                                    } else {
                                        "Le gestionnaire ne permet pas de repérer et supprimer toutes les données avec certitude."
                                    }),
                                    11.,
                                    400.,
                                    c(theme::MUTED),
                                )),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if can_remove && !busy {
                                let s = state(this);
                                s.remove_data = !s.remove_data;
                                cx.notify();
                            }
                        })),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(components::button("removal-cancel", ButtonKind::Secondary, false, None, tx("Annuler"), busy).on_click(cx.listener(move |this, _, _, cx| {
                            if !busy {
                                state(this).removal = false;
                                cx.notify();
                            }
                        })))
                        .child({
                            let target = selected.clone();
                            button_with(
                                "removal-confirm",
                                ButtonKind::Primary,
                                false,
                                Some(if busy { spinner(16., primary) } else { icon("trash-2", 16., "2", primary).into_any_element() }),
                                tx("Confirmer la désinstallation"),
                                busy,
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if busy {
                                    return;
                                }
                                let remove_data = state(this).remove_data;
                                state(this).busy = target.id.clone();
                                this.call(
                                    "application_action",
                                    json!({"id": target.id, "action": "uninstall", "remove_data": remove_data, "confirmed": true}),
                                    cx,
                                    |this, result: Result<Value, String>, cx| {
                                        state(this).busy.clear();
                                        match result {
                                            Ok(_) => {
                                                let s = state(this);
                                                s.selected = None;
                                                s.removal = false;
                                                this.notify(tx("Désinstallation terminée."), false, cx);
                                                load(this, cx);
                                            }
                                            Err(error) => this.notify(error, true, cx),
                                        }
                                    },
                                );
                            }))
                        }),
                )
                .into_any_element()
        } else {
            let (launch, reveal, preview) = (selected.clone(), selected.clone(), selected.clone());
            div()
                .flex()
                .gap(px(10.))
                .child(
                    button_with("app-launch", ButtonKind::Primary, false, Some(icon("play", 16., "2", primary).into_any_element()), tx("Lancer"), busy || !launchable)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !busy && launchable {
                                act(this, launch.clone(), "launch", cx);
                            }
                        })),
                )
                .child(
                    button_with("app-reveal", ButtonKind::Secondary, false, Some(icon("folder-open", 16., "2", secondary).into_any_element()), tx("Ouvrir le dossier"), busy || reveal.location.is_none())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !busy && reveal.location.is_some() {
                                act(this, reveal.clone(), "reveal", cx);
                            }
                        })),
                )
                .child(
                    button_with(
                        "app-uninstall",
                        ButtonKind::Secondary,
                        false,
                        Some(if busy { spinner(16., secondary) } else { icon("trash-2", 16., "2", secondary).into_any_element() }),
                        tx("Désinstaller"),
                        busy || !preview.can_uninstall,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if busy || !preview.can_uninstall {
                            return;
                        }
                        state(this).busy = preview.id.clone();
                        this.call(
                            "application_action",
                            json!({"id": preview.id, "action": "preview_uninstall"}),
                            cx,
                            |this, result: Result<Value, String>, cx| {
                                state(this).busy.clear();
                                match result {
                                    Ok(value) => {
                                        let s = state(this);
                                        s.details = value["details"].as_str().unwrap_or_default().to_owned();
                                        s.remove_data = false;
                                        s.removal = true;
                                    }
                                    Err(error) => this.notify(error, true, cx),
                                }
                            },
                        );
                    })),
                )
                .into_any_element()
        })
        .into_any_element();
    Some(components::modal(
        "application",
        selected.name.clone(),
        false,
        body,
        close,
        window,
    ))
}
