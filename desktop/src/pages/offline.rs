//! Pre-boot memory test assistant (former OfflineDiagnostic.tsx).
use crate::{
    app::{Computy, DiagnosticMode},
    components::{
        ButtonKind, NoticeTone, button_foreground, button_with, notice, notice_bold, spinner,
    },
    i18n::{tp, tx},
    intl, platform,
    theme::{self, c, hex},
    ui::{icon, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct LinuxStatus {
    pub code: String,
    pub returned: bool,
    pub confirmed: bool,
    pub pending: bool,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct BootStatus {
    pub supported: bool,
    pub pending: bool,
    pub platform: String,
    pub linux: Option<LinuxStatus>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct UsbTarget {
    pub id: String,
    pub label: String,
    pub mount: String,
    pub size_bytes: f64,
}

#[derive(Default)]
pub struct OfflineState {
    pub status: Option<BootStatus>,
    pub action: Option<&'static str>,
    pub reopen: bool,
    pub usb: bool,
    pub confirmed: bool,
    pub targets: Vec<UsbTarget>,
    pub selected: String,
    pub usb_ready: bool,
    pub problem: String,
    pub help_open: bool,
    requested: bool,
}

fn reason(code: &str) -> Option<&'static str> {
    Some(match code {
        "secure_boot" => {
            "Secure Boot protège le démarrage en autorisant uniquement les programmes approuvés par votre PC. Le moteur de test fourni avec Computy n’a pas une signature reconnue par votre firmware : ce test avant démarrage est donc bloqué, avec ou sans clé USB. Cela ne signale pas une panne de RAM."
        }
        "secure_boot_unknown" => {
            "Computy ne peut pas déterminer l’état de Secure Boot. La préparation automatique est bloquée pour éviter un démarrage incompatible."
        }
        "helper_missing" => {
            "Installez le paquet Computy pour activer l’assistant système et le moteur de test intégré. Le binaire portable seul ne modifie pas le démarrage."
        }
        "payload_missing" => {
            "Le composant de test intégré est absent. Réinstallez le paquet Computy ; aucun téléchargement séparé de Memtest86+ n’est nécessaire."
        }
        "container" => "Ce parcours nécessite le PC réel. Il est désactivé dans un conteneur.",
        "architecture" => {
            "Le démarrage automatique du composant intégré est actuellement disponible sur Linux x86-64 avec GRUB."
        }
        "unsupported_boot_storage" => {
            "Le stockage de démarrage ne permet pas de garantir le retour automatique. Computy refuse les configurations Btrfs, RAID, LVM ou chiffrées pour ce parcours."
        }
        "unsupported_grub_header" => {
            "Cette configuration GRUB ne garantit pas une sélection pour un seul démarrage. Aucun changement n’a été effectué."
        }
        "entry_missing" => {
            "L’entrée Computy est absente du menu GRUB. L’installation du paquet doit créer cette entrée sans changer le système lancé par défaut."
        }
        "another_boot_pending" => {
            "Un autre démarrage ponctuel est déjà programmé. Computy le conserve et refuse de l’écraser."
        }
        "request_already_pending" => {
            "Une demande est déjà enregistrée. Consultez-la ou annulez-la avant de recommencer."
        }
        "authorization_or_helper_failed" => {
            "L’autorisation administrateur a été refusée ou l’assistant système n’a pas pu démarrer."
        }
        "usb_existing_loader" => {
            "Cette clé contient déjà un chargeur de démarrage. Computy refuse de l’écraser. Choisissez une autre clé."
        }
        "usb_not_found" => {
            "La clé sélectionnée n’est plus disponible. Recherchez les clés à nouveau."
        }
        "usb_invalid_directory" => {
            "La structure de cette clé ne permet pas une préparation sûre. Aucun fichier existant n’a été remplacé."
        }
        _ => return None,
    })
}

fn explain(error: &str) -> String {
    let code = error.trim_start_matches("boot:");
    tx(reason(code).unwrap_or(code))
}

fn state(app: &mut Computy) -> &mut OfflineState {
    app.pages.offline.get_or_insert_with(|| OfflineState {
        reopen: true,
        ..Default::default()
    })
}

fn refresh(
    app: &mut Computy,
    cx: &mut Context<Computy>,
    then: Option<fn(&mut Computy, &BootStatus, &mut Context<Computy>)>,
) {
    app.call(
        "offline_status",
        json!({}),
        cx,
        move |this, result: Result<BootStatus, String>, cx| match result {
            Ok(status) => {
                if let Some(then) = then {
                    then(this, &status, cx);
                }
                state(this).status = Some(status);
            }
            Err(error) => state(this).problem = error,
        },
    );
}

/// Run a blocking platform operation off the UI thread.
fn act(
    app: &mut Computy,
    name: &'static str,
    cx: &mut Context<Computy>,
    operation: impl FnOnce() -> Result<Value, String> + Send + 'static,
    done: impl FnOnce(&mut Computy, Value, &mut Context<Computy>) + 'static,
) {
    let s = state(app);
    s.action = Some(name);
    s.problem.clear();
    let task = cx.background_executor().spawn(async move { operation() });
    cx.spawn(async move |this, cx| {
        let result = task.await;
        let _ = this.update(cx, |this, cx| {
            state(this).action = None;
            match result {
                Ok(value) => done(this, value, cx),
                Err(error) => {
                    let message = explain(&error);
                    state(this).problem = error;
                    this.notify(message, true, cx);
                }
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

fn checkbox_row(
    id: &'static str,
    checked: bool,
    disabled: bool,
    label: &str,
    cx: &mut Context<Computy>,
    toggle: fn(&mut OfflineState),
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(5.))
        .mt(px(14.))
        .when(!disabled, |d| d.cursor_pointer())
        .child(crate::forms::native_checkbox(
            checked,
            disabled,
            hex(0x7c9c5e),
        ))
        .child(text(tx(label), 12., 400., hex(0x90a478)))
        .on_click(cx.listener(move |this, _, _, cx| {
            if !disabled {
                toggle(state(this));
                cx.notify();
            }
        }))
}

fn steps(items: &[&str]) -> impl IntoElement {
    let color = c((0x7d9268, 0xadc29a));
    div()
        .mt(px(18.))
        .pl(px(22.))
        .flex()
        .flex_col()
        .gap(px(14.))
        .children(items.iter().enumerate().map(|(index, item)| {
            div()
                .relative()
                .pl(px(5.))
                .text_size(px(12.))
                .line_height(px(20.4))
                .text_color(color)
                .child(
                    div()
                        .absolute()
                        .left(px(-18.))
                        .top_0()
                        .child(format!("{}.", index + 1)),
                )
                .child(tx(item))
        }))
}

pub fn render(app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if !state(app).requested {
        state(app).requested = true;
        refresh(app, cx, None);
    }
    let desktop = !app.backend.is_fixture() || true;
    let s = app.pages.offline.as_ref().unwrap();
    let status = s.status.clone();
    let busy = s.action.is_some();
    let windows = status.as_ref().is_some_and(|s| s.platform == "windows");
    let linux = status.as_ref().is_some_and(|s| s.platform == "linux");
    let code = status
        .as_ref()
        .and_then(|s| s.linux.as_ref())
        .map(|l| l.code.clone())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "helper_missing".into());
    let blocked = linux && code != "ready";
    let (confirmed, reopen, usb, help_open) = (s.confirmed, s.reopen, s.usb, s.help_open);
    let problem = s.problem.clone();
    let action = s.action;
    let targets = s.targets.clone();
    let selected = s.selected.clone();
    let usb_ready = s.usb_ready;
    let warn = NoticeTone::Warn;
    let warn_text = c((0x9c8454, 0xadc29a));
    let route = |id: &'static str,
                 label: &str,
                 icon_name: &str,
                 active: bool,
                 cx: &mut Context<Computy>| {
        let kind = if active {
            ButtonKind::Primary
        } else {
            ButtonKind::Secondary
        };
        button_with(
            id,
            kind,
            false,
            Some(icon(icon_name, 18., "2", button_foreground(kind)).into_any_element()),
            tx(label),
            false,
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            let s = state(this);
            s.usb = id == "route-usb";
            s.confirmed = false;
            s.problem.clear();
            cx.notify();
        }))
    };
    let small_icon = |busy_here: bool, name: &str, size: f32, color: Hsla| -> AnyElement {
        if busy_here {
            spinner(size, color)
        } else {
            icon(name, size, "2", color).into_any_element()
        }
    };
    let secondary = button_foreground(ButtonKind::Secondary);
    let primary = button_foreground(ButtonKind::Primary);
    let mut body: Vec<AnyElement> = vec![];
    if linux {
        body.push(
            div()
                .mt(px(20.))
                .flex()
                .gap(px(12.))
                .child(route(
                    "route-internal",
                    "Sans clé USB",
                    "monitor-up",
                    !usb,
                    cx,
                ))
                .child(route("route-usb", "Avec clé USB", "usb", usb, cx))
                .into_any_element(),
        );
    }
    if blocked {
        let bold = notice_bold(warn);
        let mut children = vec![
            text(
                tx(if code == "secure_boot" {
                    "Test avant démarrage bloqué par Secure Boot"
                } else {
                    "Vérification de compatibilité"
                }),
                12.,
                600.,
                bold,
            )
            .line_height(px(21.))
            .into_any_element(),
            text(tx(reason(&code).unwrap_or(&code)), 12., 400., warn_text)
                .line_height(px(19.8))
                .into_any_element(),
        ];
        if code == "secure_boot" {
            children.push(
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_col()
                    .items_start()
                    .child(
                        button_with(
                            "continue-session",
                            ButtonKind::Secondary,
                            true,
                            Some(icon("arrow-right", 15., "2", secondary).into_any_element()),
                            tx("Continuer avec un test dans ma session"),
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.diagnostic_mode = DiagnosticMode::Session;
                            cx.notify();
                        })),
                    )
                    .child(text(tx("Le test en session reste disponible sans changer le firmware. Il vérifie une partie de la RAM, pas toute la mémoire physique."), 12., 400., warn_text).line_height(px(19.8)))
                    .child(
                        div()
                            .id("secure-boot-help")
                            .mt(px(12.))
                            .cursor_pointer()
                            .child(text(tx("Comment débloquer le test avant démarrage ?"), 12., 600., warn_text).line_height(px(21.)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let s = state(this);
                                s.help_open = !s.help_open;
                                cx.notify();
                            })),
                    )
                    .when(help_open, |d| {
                        d.child(
                            div()
                                .mt(px(12.))
                                .pl(px(40.))
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .children([
                                    "Consultez le guide de votre modèle de PC pour ouvrir les réglages UEFI et vérifier si Secure Boot peut être désactivé temporairement. Computy ne change pas ce réglage.",
                                    "Avant toute modification, conservez votre clé de récupération si votre disque est chiffré, notamment avec BitLocker ou un déverrouillage TPM. Un changement de Secure Boot peut la demander. Sur un PC géré, contactez votre administrateur.",
                                    "Si vous choisissez cette option, revenez dans Computy après le redémarrage, puis cliquez sur « Revérifier la compatibilité ». Vous devrez encore confirmer la préparation du test.",
                                    "Après le test, réactivez Secure Boot dans les réglages UEFI. N’effacez pas les clés du firmware et ne remettez pas les réglages du PC à zéro.",
                                ]
                                .iter()
                                .enumerate()
                                .map(|(i, item)| {
                                    div()
                                        .relative()
                                        .text_size(px(12.))
                                        .line_height(px(19.8))
                                        .text_color(warn_text)
                                        .child(div().absolute().left(px(-18.)).child(format!("{}.", i + 1)))
                                        .child(tx(item))
                                }))
                                .child(text(tx("Une clé USB utilise le même moteur et ne supprime pas cette exigence. Les certificats du moteur original ne valident pas le binaire modifié de Computy."), 12., 400., warn_text).line_height(px(19.8)).ml(px(-40.))),
                        )
                    })
                    .into_any_element(),
            );
        }
        children.push(
            div()
                .mt(px(14.))
                .flex()
                .child(
                    button_with(
                        "verify-boot",
                        ButtonKind::Secondary,
                        true,
                        Some(small_icon(action == Some("verify"), "refresh-cw", 15., secondary)),
                        tx("Revérifier la compatibilité"),
                        busy,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if busy {
                            return;
                        }
                        state(this).action = Some("verify");
                        refresh(this, cx, Some(|this, status, cx| {
                            state(this).action = None;
                            let ready = status.linux.as_ref().is_some_and(|l| l.code == "ready");
                            this.notify(
                                tx(if ready {
                                    "Compatibilité vérifiée : la préparation est disponible."
                                } else {
                                    "Vérification terminée : consultez les conditions affichées ci-dessus."
                                }),
                                false,
                                cx,
                            );
                        }));
                        cx.notify();
                    })),
                )
                .into_any_element(),
        );
        body.push(
            notice(warn, children)
                .text_size(px(12.))
                .line_height(px(21.))
                .mt(px(20.))
                .into_any_element(),
        );
    }
    if status.is_none() && problem.is_empty() {
        body.push(
            div()
                .mt(px(18.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(spinner(16., c(theme::MUTED)))
                .child(text(
                    tx("Vérification de compatibilité…"),
                    12.,
                    400.,
                    c(theme::MUTED),
                ))
                .into_any_element(),
        );
    }
    if !problem.is_empty() {
        body.push(
            notice(warn, vec![explain(&problem).into_any_element()])
                .mt(px(18.))
                .into_any_element(),
        );
    }
    if let Some(status) = status.clone() {
        let linux_status = status.linux.clone().unwrap_or_default();
        if windows {
            body.push(steps(&[
                "Computy ouvre le diagnostic mémoire de Windows. Vous choisissez le prochain redémarrage dans le dialogue système.",
                "Le test s’exécute avant le retour sur le bureau, puis Windows redémarre.",
                "À l’ouverture de Computy, un nouvel événement de résultat est récupéré et ajouté à l’historique.",
            ]).into_any_element());
            if status.pending {
                body.push(
                    div()
                        .mt(px(18.))
                        .flex()
                        .child(
                            button_with("collect", ButtonKind::Secondary, false, Some(icon("refresh-cw", 16., "2", secondary).into_any_element()), tx("Rechercher le résultat"), busy)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if busy {
                                        return;
                                    }
                                    this.call("offline_collect", json!({}), cx, |this, result: Result<Value, String>, cx| match result {
                                        Ok(value) => {
                                            refresh(this, cx, None);
                                            this.reload_history(cx);
                                            let imported = value["imported"] == true;
                                            this.notify(
                                                if imported {
                                                    tx("Le résultat Windows a été ajouté à l’historique.")
                                                } else {
                                                    tx(value["reason"].as_str().unwrap_or("Le résultat du diagnostic n’est pas encore disponible."))
                                                },
                                                false,
                                                cx,
                                            );
                                        }
                                        Err(error) => this.notify(explain(&error), true, cx),
                                    });
                                })),
                        )
                        .into_any_element(),
                );
            } else {
                body.push(
                    checkbox_row(
                        "reopen",
                        reopen,
                        false,
                        "Rouvrir Computy à la connexion pour retrouver le résultat",
                        cx,
                        |s| s.reopen = !s.reopen,
                    )
                    .into_any_element(),
                );
                body.push(
                    launch_button(
                        app,
                        "Ouvrir le diagnostic Windows",
                        "monitor-up",
                        busy || !desktop,
                        cx,
                    )
                    .into_any_element(),
                );
            }
        } else if linux && !usb {
            body.push(steps(&[
                "Computy vérifie le moteur intégré, Secure Boot, GRUB et la possibilité de revenir au système habituel.",
                "Après votre accord et l’autorisation administrateur, seule l’entrée du prochain démarrage change. Aucune clé USB n’est nécessaire.",
                "Vous redémarrez quand vous êtes prêt. Le moteur Computy vérifie la RAM avant Linux ; Échap permet de quitter et redémarrer.",
                "Le démarrage suivant revient au système habituel. Les résultats Linux ne sont pas importés automatiquement : notez le résultat affiché. Une absence de rapport ne devient jamais un succès.",
            ]).into_any_element());
            if status.pending {
                body.push(
                    notice(
                        NoticeTone::Info,
                        vec![tx(if linux_status.returned {
                            "Vous êtes revenu dans le système. Computy n’a pas de preuve automatique du résultat affiché avant démarrage."
                        } else {
                            "Une préparation est enregistrée. Vous pouvez encore l’annuler."
                        })
                        .into_any_element()],
                    )
                    .mt(px(18.))
                    .into_any_element(),
                );
                let can_reboot = !busy && !blocked && linux_status.confirmed && confirmed;
                body.push(
                    div()
                        .mt(px(18.))
                        .flex()
                        .gap(px(12.))
                        .child(
                            button_with(
                                "cancel-boot",
                                ButtonKind::Secondary,
                                false,
                                None,
                                tx("Annuler la préparation"),
                                busy,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if busy {
                                        return;
                                    }
                                    act(
                                        this,
                                        "operation",
                                        cx,
                                        || platform::boot_action("cancel"),
                                        |this, _, cx| {
                                            refresh(this, cx, None);
                                            this.notify(
                                                tx("Préparation du prochain démarrage annulée."),
                                                false,
                                                cx,
                                            );
                                        },
                                    );
                                },
                            )),
                        )
                        .when(!linux_status.returned, |d| {
                            d.child(
                                button_with(
                                    "reboot",
                                    ButtonKind::Primary,
                                    false,
                                    Some(icon("power", 16., "2", primary).into_any_element()),
                                    tx("Redémarrer maintenant"),
                                    !can_reboot,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if can_reboot {
                                            act(
                                                this,
                                                "operation",
                                                cx,
                                                || platform::boot_action("reboot"),
                                                |_, _, _| {},
                                            );
                                        }
                                    },
                                )),
                            )
                        })
                        .into_any_element(),
                );
                if !linux_status.returned {
                    body.push(
                        checkbox_row(
                            "confirm-reboot",
                            confirmed,
                            false,
                            "J’ai enregistré mon travail et je confirme le redémarrage de ce PC.",
                            cx,
                            |s| s.confirmed = !s.confirmed,
                        )
                        .into_any_element(),
                    );
                }
            } else {
                body.push(
                    checkbox_row(
                        "reopen",
                        reopen,
                        false,
                        "Rouvrir Computy à la connexion pour retrouver le résultat",
                        cx,
                        |s| s.reopen = !s.reopen,
                    )
                    .into_any_element(),
                );
                body.push(checkbox_row("confirm-prepare", confirmed, blocked || !desktop, "J’autorise Computy à préparer un seul démarrage de test. Cela ne redémarre pas le PC maintenant.", cx, |s| s.confirmed = !s.confirmed).into_any_element());
                body.push(
                    launch_button(
                        app,
                        "Préparer le prochain démarrage",
                        "shield-check",
                        blocked || busy || !confirmed || !desktop,
                        cx,
                    )
                    .into_any_element(),
                );
            }
        } else if linux {
            body.push(steps(&[
                "Branchez une clé USB déjà formatée en FAT32. Computy recherche uniquement les volumes USB montés et exclut le disque qui contient le système.",
                "Choisissez la clé, vérifiez son nom et son emplacement, puis confirmez. Computy copie son moteur sans formater la clé ni remplacer un chargeur existant.",
                "Après confirmation, redémarrez et choisissez la clé dans le menu de démarrage du firmware. La touche dépend du modèle du PC ; elle est souvent indiquée à l’allumage.",
                "Le moteur affiche ses résultats. F6 peut enregistrer un rapport sur une clé FAT32 ; Échap quitte le test. Le retour dans Computy ne signifie pas automatiquement que le test a réussi.",
            ]).into_any_element());
            let searching = action == Some("usb-search");
            body.push(
                div()
                    .mt(px(18.))
                    .flex()
                    .child(
                        button_with(
                            "usb-search",
                            ButtonKind::Secondary,
                            false,
                            Some(small_icon(searching, "refresh-cw", 16., secondary)),
                            tx(if searching { "Recherche des clés USB…" } else { "Rechercher les clés compatibles" }),
                            busy || !desktop,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if busy {
                                return;
                            }
                            let s = state(this);
                            s.usb_ready = false;
                            s.targets.clear();
                            s.selected.clear();
                            s.confirmed = false;
                            act(this, "usb-search", cx, platform::usb_targets, |this, value, cx| {
                                let found: Vec<UsbTarget> = serde_json::from_value(value).unwrap_or_default();
                                let count = found.len();
                                state(this).targets = found;
                                this.notify(
                                    if count > 0 {
                                        tp("Clés USB compatibles trouvées : {{count}}", &[("count", count.into())])
                                    } else {
                                        tx("Aucune clé USB compatible trouvée. Branchez et montez une clé FAT32, puis réessayez.")
                                    },
                                    false,
                                    cx,
                                );
                            });
                        })),
                    )
                    .into_any_element(),
            );
            let locale = crate::i18n::locale();
            body.push(
                div()
                    .mt(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(targets.iter().map(|target| {
                        let id = target.id.clone();
                        let checked = selected == target.id;
                        div()
                            .id(SharedString::from(format!("usb-{}", target.id)))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .p(px(12.))
                            .border_1()
                            .border_color(c(theme::BORDER))
                            .rounded(px(9.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .size(px(13.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if checked {
                                        hex(0x0075ff)
                                    } else {
                                        hex(0x767676)
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(checked, |d| {
                                        d.child(div().size(px(7.)).rounded_full().bg(hex(0x0075ff)))
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(text(target.label.clone(), 12., 700., c(theme::TEXT)))
                                    .child(text(
                                        format!(
                                            "{} · {}",
                                            target.mount,
                                            intl::bytes(locale, Some(target.size_bytes), 1)
                                        ),
                                        11.,
                                        400.,
                                        c(theme::MUTED),
                                    )),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let s = state(this);
                                s.selected = id.clone();
                                s.confirmed = false;
                                s.usb_ready = false;
                                cx.notify();
                            }))
                    }))
                    .into_any_element(),
            );
            if targets.is_empty() {
                body.push(
                    text(tx("Si aucune clé n’apparaît, montez une clé FAT32 ou utilisez le parcours sans clé. Computy ne formate aucun disque automatiquement."), 12., 400., c((0x7d9268, 0xadc29a)))
                        .line_height(px(22.2))
                        .mt(px(12.))
                        .into_any_element(),
                );
            }
            if let Some(target) = targets.iter().find(|t| t.id == selected).cloned() {
                let destination = format!("{} : ", tx("Destination vérifiée"));
                let tail = format!(" — {}", target.mount);
                body.push(
                    notice(
                        NoticeTone::Info,
                        vec![
                            crate::components::rich(&[(&destination, false), (&target.label, true), (&tail, false)], notice_bold(NoticeTone::Info)).into_any_element(),
                            tx("Fichier ajouté : EFI/BOOT/BOOTX64.EFI. Les autres fichiers restent en place. Une clé avec un chargeur existant est refusée.").into_any_element(),
                        ],
                    )
                    .mt(px(18.))
                    .into_any_element(),
                );
                body.push(
                    checkbox_row(
                        "confirm-usb",
                        confirmed,
                        blocked,
                        "Je confirme l’écriture du moteur Computy sur cette clé USB.",
                        cx,
                        |s| s.confirmed = !s.confirmed,
                    )
                    .into_any_element(),
                );
                let disabled = busy || blocked || !confirmed;
                let id = target.id.clone();
                body.push(
                    div()
                        .mt(px(14.))
                        .flex()
                        .child(
                            button_with("prepare-usb", ButtonKind::Primary, false, Some(small_icon(busy, "usb", 16., primary)), tx("Préparer cette clé USB"), disabled)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if disabled {
                                        return;
                                    }
                                    let id = id.clone();
                                    act(this, "operation", cx, move || platform::prepare_usb(&id), |this, _, cx| {
                                        state(this).usb_ready = true;
                                        this.notify(tx("Clé USB préparée. Vous pouvez maintenant l’éjecter proprement."), false, cx);
                                    });
                                })),
                        )
                        .into_any_element(),
                );
            }
            if usb_ready {
                body.push(
                    notice(NoticeTone::Info, vec![tx("Clé préparée. Éjectez-la proprement, puis choisissez-la dans le menu de démarrage de votre PC. Aucun redémarrage n’a été déclenché.").into_any_element()])
                        .mt(px(18.))
                        .into_any_element(),
                );
            }
        } else {
            body.push(
                notice(NoticeTone::Info, vec![tx("Sur Mac, Apple Diagnostics doit être lancé avec les commandes de démarrage du modèle. Cette version ne programme pas ce parcours et n’importe pas les résultats Apple.").into_any_element()])
                    .mt(px(18.))
                    .into_any_element(),
            );
        }
    }
    div()
        .flex()
        .items_start()
        .gap(px(24.))
        .p(px(28.))
        .bg(c((0xf0f4e9, 0x253322)))
        .border_1()
        .border_color(c((0xe2e8da, 0x394a2e)))
        .rounded(px(14.))
        .child(
            div()
                .size(px(49.))
                .flex_none()
                .rounded(px(13.))
                .bg(c((0xe5edd9, 0x304129)))
                .border_1()
                .border_color(c((0xdae4cb, 0x405333)))
                .flex()
                .items_center()
                .justify_center()
                .child(icon("power", 25., "2", c((0x9cae80, 0xa4c580)))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .max_w(px(810.))
                .flex()
                .flex_col()
                .child(spaced(tx("TEST AVANT DÉMARRAGE"), 10., 650., hex(0x899582), 1.75, 13.).mt(px(6.)))
                .child(spaced(tx("Tester la mémoire avant le lancement du système"), 20., 650., c(theme::HEADING), -0.6, 26.).mt(px(2.)))
                .child(
                    text(tx("Computy prépare le parcours. Vous choisissez le support et le moment du redémarrage. Aucun test au démarrage ne part depuis un profil de session."), 12., 400., c((0x7d9268, 0xadc29a)))
                        .line_height(px(22.2))
                        .mt(px(12.)),
                )
                .children(body)
                .child(
                    text(tx("Moteur avant démarrage intégré : dérivé de Memtest86+ 9.00, sous GPLv2. Les sources correspondantes sont fournies avec le paquet Computy."), 12., 400., c((0x7d9268, 0xadc29a)))
                        .line_height(px(22.2))
                        .max_w(px(650.))
                        .mt(px(13.)),
                ),
        )
        .into_any_element()
}

fn launch_button(
    app: &Computy,
    label: &str,
    icon_name: &str,
    disabled: bool,
    cx: &mut Context<Computy>,
) -> impl IntoElement {
    let busy = app
        .pages
        .offline
        .as_ref()
        .is_some_and(|s| s.action.is_some());
    let primary = button_foreground(ButtonKind::Primary);
    let lead = if busy {
        spinner(16., primary)
    } else {
        icon(icon_name, 16., "2", primary).into_any_element()
    };
    div().mt(px(14.)).flex().child(
        button_with("prepare-boot", ButtonKind::Primary, false, Some(lead), tx(label), disabled).on_click(cx.listener(move |this, _, _, cx| {
            if disabled {
                return;
            }
            let Some(service) = this.backend.service().cloned() else { return };
            let reopen = state(this).reopen;
            let windows = state(this).status.as_ref().is_some_and(|s| s.platform == "windows");
            act(
                this,
                "operation",
                cx,
                move || {
                    platform::open_offline_tool(&service)?;
                    let startup = if reopen { platform::set_startup(&service, true).err() } else { None };
                    Ok(json!({ "startup_error": startup }))
                },
                move |this, value, cx| {
                    if let Some(error) = value["startup_error"].as_str() {
                        this.notify(error.to_owned(), true, cx);
                    }
                    refresh(this, cx, None);
                    state(this).confirmed = false;
                    this.notify(
                        tx(if windows {
                            "Le dialogue Windows est ouvert. Choisissez vous-même quand redémarrer."
                        } else {
                            "Le prochain démarrage sélectionnera Computy une seule fois. Le système par défaut reste inchangé."
                        }),
                        false,
                        cx,
                    );
                },
            );
        })),
    )
}
