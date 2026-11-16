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
        "secure_boot" => "offline.secure_boot_protects_startup_allowing",
        "secure_boot_unknown" => "offline.computy_cannot_determine_state_secure",
        "helper_missing" => "offline.install_computy_package_enable_system",
        "payload_missing" => "offline.built_test_component_missing_reinstall",
        "container" => "offline.route_requires_real_pc_disabled",
        "architecture" => "offline.automatic_start_integrated_component_currently",
        "unsupported_boot_storage" => "offline.boot_storage_does_not_guarantee",
        "unsupported_grub_header" => "offline.grub_configuration_does_not_guarantee",
        "entry_missing" => "offline.computy_entry_missing_grub_menu",
        "another_boot_pending" => "offline.another_one_time_start_already",
        "request_already_pending" => "offline.application_already_registered_consult_cancel",
        "authorization_or_helper_failed" => {
            "offline.administrator_authorization_was_refused_system"
        }
        "usb_existing_loader" => "offline.key_already_contains_boot_charger",
        "usb_not_found" => "offline.selected_key_no_longer_available",
        "usb_invalid_directory" => "offline.structure_key_does_not_allow",
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
                    "offline.without_usb_drive",
                    "monitor-up",
                    !usb,
                    cx,
                ))
                .child(route("route-usb", "offline.usb_drive", "usb", usb, cx))
                .into_any_element(),
        );
    }
    if blocked {
        let bold = notice_bold(warn);
        let mut children = vec![
            text(
                tx(if code == "secure_boot" {
                    "offline.pre_boot_test_blocked_secure"
                } else {
                    "offline.compatibility_check"
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
                            tx("offline.continue_session_test"),
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.diagnostic_mode = DiagnosticMode::Session;
                            cx.notify();
                        })),
                    )
                    .child(
                        text(
                            tx("offline.session_test_remains_available_without"),
                            12.,
                            400.,
                            warn_text,
                        )
                        .line_height(px(19.8)),
                    )
                    .child(
                        div()
                            .id("secure-boot-help")
                            .mt(px(12.))
                            .cursor_pointer()
                            .child(
                                text(tx("offline.how_to_unblock_preboot"), 12., 600., warn_text)
                                    .line_height(px(21.)),
                            )
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
                                .children(
                                    [
                                        "offline.check_pc_models_guide_open",
                                        "offline.before_changing_anything_keep_recovery",
                                        "offline.if_choose_option_return_computy",
                                        "offline.after_test_re_enable_secure",
                                    ]
                                    .iter()
                                    .enumerate()
                                    .map(|(i, item)| {
                                        div()
                                            .relative()
                                            .text_size(px(12.))
                                            .line_height(px(19.8))
                                            .text_color(warn_text)
                                            .child(
                                                div()
                                                    .absolute()
                                                    .left(px(-18.))
                                                    .child(format!("{}.", i + 1)),
                                            )
                                            .child(tx(item))
                                    }),
                                )
                                .child(
                                    text(
                                        tx("offline.usb_drive_uses_same_engine"),
                                        12.,
                                        400.,
                                        warn_text,
                                    )
                                    .line_height(px(19.8))
                                    .ml(px(-40.)),
                                ),
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
                        Some(small_icon(
                            action == Some("verify"),
                            "refresh-cw",
                            15.,
                            secondary,
                        )),
                        tx("offline.check_compatibility_again"),
                        busy,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if busy {
                            return;
                        }
                        state(this).action = Some("verify");
                        refresh(
                            this,
                            cx,
                            Some(|this, status, cx| {
                                state(this).action = None;
                                let ready =
                                    status.linux.as_ref().is_some_and(|l| l.code == "ready");
                                this.notify(
                                    tx(if ready {
                                        "offline.compatibility_checked_preparation_available"
                                    } else {
                                        "offline.check_complete_review_requirements_above"
                                    }),
                                    false,
                                    cx,
                                );
                            }),
                        );
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
                    tx("offline.checking_compatibility"),
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
            body.push(
                steps(&[
                    "offline.computy_opens_windows_memory_diagnostic",
                    "offline.test_runs_before_returning_desktop",
                    "offline.opening_computy_new_result_event",
                ])
                .into_any_element(),
            );
            if status.pending {
                body.push(
                    div()
                        .mt(px(18.))
                        .flex()
                        .child(
                            button_with("collect", ButtonKind::Secondary, false, Some(icon("refresh-cw", 16., "2", secondary).into_any_element()), tx("offline.find_result"), busy)
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
                                                    tx("offline.windows_result_was_added_history")
                                                } else {
                                                    tx(value["reason"].as_str().unwrap_or("offline.diagnostic_results_not_yet_available"))
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
                        "offline.reopen_computy_connection_find_result",
                        cx,
                        |s| s.reopen = !s.reopen,
                    )
                    .into_any_element(),
                );
                body.push(
                    launch_button(
                        app,
                        "offline.open_windows_diagnostic",
                        "monitor-up",
                        busy || !desktop,
                        cx,
                    )
                    .into_any_element(),
                );
            }
        } else if linux && !usb {
            body.push(
                steps(&[
                    "offline.computy_checks_built_engine_secure",
                    "offline.after_agreement_admin_authorization_only",
                    "offline.restart_when_ready_computy_engine",
                    "offline.next_start_usual_system_linux",
                ])
                .into_any_element(),
            );
            if status.pending {
                body.push(
                    notice(
                        NoticeTone::Info,
                        vec![
                            tx(if linux_status.returned {
                                "offline.have_returned_system_computy_has"
                            } else {
                                "offline.preparation_registered_still_cancel"
                            })
                            .into_any_element(),
                        ],
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
                                tx("offline.cancel_preparation"),
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
                                                tx("offline.next_boot_preparation_cancelled"),
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
                                    tx("offline.restart_now"),
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
                            "offline.i_saved_my_work_confirmed",
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
                        "offline.reopen_computy_connection_find_result",
                        cx,
                        |s| s.reopen = !s.reopen,
                    )
                    .into_any_element(),
                );
                body.push(
                    checkbox_row(
                        "confirm-prepare",
                        confirmed,
                        blocked || !desktop,
                        "offline.i_ll_allow_computy_prepare",
                        cx,
                        |s| s.confirmed = !s.confirmed,
                    )
                    .into_any_element(),
                );
                body.push(
                    launch_button(
                        app,
                        "offline.prepare_next_boot",
                        "shield-check",
                        blocked || busy || !confirmed || !desktop,
                        cx,
                    )
                    .into_any_element(),
                );
            }
        } else if linux {
            body.push(
                steps(&[
                    "offline.plug_usb_drive_already_formatted",
                    "offline.choose_key_check_name_location",
                    "offline.after_confirmation_restart_select_key",
                    "offline.engine_displays_results_f6_record",
                ])
                .into_any_element(),
            );
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
                            tx(if searching {
                                "offline.searching_usb_drives"
                            } else {
                                "offline.search_compatible_usb_drives"
                            }),
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
                            act(
                                this,
                                "usb-search",
                                cx,
                                platform::usb_targets,
                                |this, value, cx| {
                                    let found: Vec<UsbTarget> =
                                        serde_json::from_value(value).unwrap_or_default();
                                    let count = found.len();
                                    state(this).targets = found;
                                    this.notify(
                                        if count > 0 {
                                            tp(
                                                "offline.compatible_usb_drives_found",
                                                &[("count", count.into())],
                                            )
                                        } else {
                                            tx("offline.no_compatible_usb_drive_found")
                                        },
                                        false,
                                        cx,
                                    );
                                },
                            );
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
                    text(
                        tx("offline.if_no_key_appears_mount"),
                        12.,
                        400.,
                        c((0x7d9268, 0xadc29a)),
                    )
                    .line_height(px(22.2))
                    .mt(px(12.))
                    .into_any_element(),
                );
            }
            if let Some(target) = targets.iter().find(|t| t.id == selected).cloned() {
                let destination = format!("{} : ", tx("offline.destination_verified"));
                let tail = format!(" — {}", target.mount);
                body.push(
                    notice(
                        NoticeTone::Info,
                        vec![
                            crate::components::rich(
                                &[(&destination, false), (&target.label, true), (&tail, false)],
                                notice_bold(NoticeTone::Info),
                            )
                            .into_any_element(),
                            tx("offline.added_file_efi_boot_bootx64").into_any_element(),
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
                        "offline.i_confirm_engine_writing_computy",
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
                            button_with(
                                "prepare-usb",
                                ButtonKind::Primary,
                                false,
                                Some(small_icon(busy, "usb", 16., primary)),
                                tx("offline.prepare_usb_flash_drive"),
                                disabled,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if disabled {
                                        return;
                                    }
                                    let id = id.clone();
                                    act(
                                        this,
                                        "operation",
                                        cx,
                                        move || platform::prepare_usb(&id),
                                        |this, _, cx| {
                                            state(this).usb_ready = true;
                                            this.notify(
                                                tx("offline.usb_drive_prepared_now_safely"),
                                                false,
                                                cx,
                                            );
                                        },
                                    );
                                },
                            )),
                        )
                        .into_any_element(),
                );
            }
            if usb_ready {
                body.push(
                    notice(
                        NoticeTone::Info,
                        vec![tx("offline.prepared_key_eject_cleanly_choose").into_any_element()],
                    )
                    .mt(px(18.))
                    .into_any_element(),
                );
            }
        } else {
            body.push(
                notice(
                    NoticeTone::Info,
                    vec![tx("offline.mac_apple_diagnostics_must_launched").into_any_element()],
                )
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
                .child(
                    spaced(
                        tx("offline.preboot_test_eyebrow"),
                        10.,
                        650.,
                        hex(0x899582),
                        1.75,
                        13.,
                    )
                    .mt(px(6.)),
                )
                .child(
                    spaced(
                        tx("offline.test_memory_before_system_starts"),
                        20.,
                        650.,
                        c(theme::HEADING),
                        -0.6,
                        26.,
                    )
                    .mt(px(2.)),
                )
                .child(
                    text(
                        tx("offline.computy_prepares_course_choose_support"),
                        12.,
                        400.,
                        c((0x7d9268, 0xadc29a)),
                    )
                    .line_height(px(22.2))
                    .mt(px(12.)),
                )
                .children(body)
                .child(
                    text(
                        tx("offline.integrated_pre_start_engine_derived"),
                        12.,
                        400.,
                        c((0x7d9268, 0xadc29a)),
                    )
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
        button_with(
            "prepare-boot",
            ButtonKind::Primary,
            false,
            Some(lead),
            tx(label),
            disabled,
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            if disabled {
                return;
            }
            let Some(service) = this.backend.service().cloned() else {
                return;
            };
            let reopen = state(this).reopen;
            let windows = state(this)
                .status
                .as_ref()
                .is_some_and(|s| s.platform == "windows");
            act(
                this,
                "operation",
                cx,
                move || {
                    platform::open_offline_tool(&service)?;
                    let startup = if reopen {
                        platform::set_startup(&service, true).err()
                    } else {
                        None
                    };
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
                            "offline.windows_dialog_open_choose_when"
                        } else {
                            "offline.next_boot_select_computy_only"
                        }),
                        false,
                        cx,
                    );
                },
            );
        })),
    )
}
