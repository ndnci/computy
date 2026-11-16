//! Memory, storage and processor pages (former pages/Hardware.tsx).
use crate::{
    app::{Computy, Page},
    components::{
        self, ButtonKind, NoticeTone, button, check_card, component_summary, empty, neutral_badge,
        notice, notice_bold, rich, score, spinner, text_button,
    },
    i18n::{tn, tp, tx},
    intl,
    theme::{self, c, hex},
    ui::{icon, one_line, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};

pub fn stat(label: &str, value: String, foot: String) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .p(px(20.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(13.))
        .child(text(tx(label), 12., 400., hex(0x8c9880)))
        .child(spaced(tx(&value), 23., 550., c(theme::TEXT), -0.6, 32.2).mt(px(12.)))
        .child(
            text(tx(&foot), 11., 400., hex(0x81936d))
                .line_height(px(18.15))
                .mt(px(8.)),
        )
}

pub fn spec_row(label: String, value: AnyElement) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap(px(18.))
        .py(px(12.))
        .border_t_1()
        .border_color(c(theme::BORDER))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(text(label, 12., 400., hex(0x7b9069)).line_height(px(19.2))),
        )
        .child(
            div()
                .flex_none()
                .max_w(relative(0.6))
                .text_size(px(12.))
                .font_weight(FontWeight(500.))
                .line_height(px(19.2))
                .text_color(hex(0x70825c))
                .text_right()
                .child(value),
        )
}

pub fn note(content: String) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap(px(10.))
        .px(px(23.))
        .pb(px(16.))
        .text_size(px(12.))
        .line_height(px(21.6))
        .text_color(c((0x7d9169, 0xadc29a)))
        .child(icon("circle-help", 15., "2", hex(0xa3b78d)).mt(px(2.)))
        .child(div().flex_1().min_w_0().child(content))
}

pub fn two_col(left: impl IntoElement, right: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .gap(px(20.))
        .mb(px(20.))
        .child(div().flex_1().min_w_0().flex().child(left))
        .child(div().flex_1().min_w_0().flex().child(right))
}

fn half_panel() -> Div {
    panel()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .overflow_hidden()
}

fn refresh_button(app: &Computy, cx: &mut Context<Computy>) -> AnyElement {
    let refreshing = app.refreshing;
    let color = c((0x65715f, 0xb1c699));
    div()
        .id("banner-refresh")
        .flex()
        .flex_none()
        .items_center()
        .gap(px(7.))
        .py(px(8.))
        .px(px(11.))
        .min_h(px(32.))
        .rounded(px(8.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c((0xe1e6dd, 0x425435)))
        .when(refreshing, |d| d.opacity(0.5))
        .when(!refreshing, |d| {
            d.cursor_pointer().hover(|s| s.bg(c((0xf1f5eb, 0x35492c))))
        })
        .child(if refreshing {
            spinner(14., color)
        } else {
            icon("refresh-cw", 14., "2", color).into_any_element()
        })
        .child(text(tx("common.refresh"), 11., 600., color).line_height(px(13.2)))
        .on_click(cx.listener(move |this, _, _, cx| {
            if !refreshing {
                this.refresh(true, cx)
            }
        }))
        .into_any_element()
}

fn banner(app: &Computy, icon_name: &str, cx: &mut Context<Computy>) -> impl IntoElement {
    let inventory = app.inventory.as_ref().unwrap();
    let title = if inventory.model.is_empty() {
        &inventory.host_name
    } else {
        &inventory.model
    };
    div()
        .flex()
        .items_center()
        .gap(px(13.))
        .py(px(17.))
        .px(px(21.))
        .mb(px(22.))
        .bg(c((0xf0f4e8, 0x253322)))
        .border_1()
        .border_color(c((0xe2e8da, 0x394a2e)))
        .rounded(px(12.))
        .child(
            div()
                .size(px(40.))
                .flex_none()
                .rounded(px(9.))
                .bg(c((0xe5edda, 0x304129)))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(icon_name, 23., "2", c((0x88a06a, 0xa4c580)))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(text(tx(title), 13., 600., c(theme::TEXT)))
                .child(text(
                    format!(
                        "{} {}{}{}",
                        tx(&inventory.os),
                        tx(&inventory.os_version),
                        tx(" · "),
                        tx(&inventory.arch)
                    ),
                    10.,
                    400.,
                    hex(0x92a17d),
                )),
        )
        .child(refresh_button(app, cx))
}

fn memory(app: &Computy) -> Vec<AnyElement> {
    let locale = crate::i18n::locale();
    let inventory = app.inventory.as_ref().unwrap();
    let memory = &inventory.memory;
    let bytes = |v: f64| intl::bytes(locale, Some(v), 1);
    let not_given = |key: &str| tx(key);
    let modules = if memory.modules.is_empty() {
        empty(
            "memory-stick",
            tx("hardware.system_keeps_details_private"),
            Some(tx("hardware.amount_ram_exposed_os_but").into_any_element()),
            None,
        )
        .into_any_element()
    } else {
        div()
            .flex()
            .flex_wrap()
            .gap(px(16.))
            .px(px(23.))
            .pb(px(23.))
            .children(memory.modules.iter().enumerate().map(|(index, module)| {
                let speed = |value: Option<f64>| {
                    value.map_or_else(
                        || tx("common.not_reported"),
                        |v| format!("{} MT/s", intl::plain(locale, v)),
                    )
                };
                let replace = match module.replaceable {
                    Some(true) => div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_color(c(theme::GOOD))
                        .child(icon("wrench", 13., "2", c(theme::GOOD)))
                        .child(tx("hardware.generally_replaceable_format"))
                        .into_any_element(),
                    Some(false) => tx("hardware.integrated_soldered_component").into_any_element(),
                    None => tx("hardware.confirmed_manufacturer").into_any_element(),
                };
                div()
                    .w(px(350.))
                    .flex()
                    .flex_col()
                    .p(px(20.))
                    .border_1()
                    .border_color(c(theme::BORDER))
                    .rounded(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .w(px(60.))
                                    .h(px(30.))
                                    .rounded(px(4.))
                                    .bg(c((0xe5edda, 0x304129))),
                            )
                            .child(neutral_badge(if module.locator.is_empty() {
                                tx(&format!("Module {}", index + 1))
                            } else {
                                tx(&module.locator)
                            })),
                    )
                    .child(
                        div()
                            .mt(px(14.))
                            .flex()
                            .items_baseline()
                            .gap(px(6.))
                            .child(text(
                                bytes(module.capacity_bytes),
                                20.,
                                650.,
                                c(theme::HEADING),
                            ))
                            .child(text(tx(&module.kind), 12., 500., c(theme::MUTED))),
                    )
                    .child(text(
                        if module.manufacturer.is_empty() {
                            tx("hardware.manufacturer_not_communicated")
                        } else {
                            tx(&module.manufacturer)
                        },
                        12.,
                        400.,
                        c(theme::SOFT_TEXT),
                    ))
                    .child(
                        div()
                            .mt(px(10.))
                            .flex()
                            .flex_col()
                            .child(spec_row(
                                tx("common.reference"),
                                (if module.part_number.is_empty() {
                                    tx("common.not_reported_feminine")
                                } else {
                                    tx(&module.part_number)
                                })
                                .into_any_element(),
                            ))
                            .child(spec_row(
                                tx("hardware.format"),
                                (if module.form_factor.is_empty() {
                                    tx("common.not_reported")
                                } else {
                                    tx(&module.form_factor)
                                })
                                .into_any_element(),
                            ))
                            .child(spec_row(
                                tx("hardware.module_rated_speed"),
                                speed(module.speed_mts).into_any_element(),
                            ))
                            .child(spec_row(
                                tx("hardware.configured_speed"),
                                speed(module.configured_speed_mts).into_any_element(),
                            ))
                            .child(spec_row(tx("hardware.replacement"), replace)),
                    )
            }))
            .into_any_element()
    };
    let none = tx("hardware.none");
    let apple = inventory.os.to_lowercase().contains("mac") && inventory.arch == "aarch64";
    vec![
        div()
            .flex()
            .gap(px(16.))
            .mb(px(20.))
            .child(stat(
                "hardware.memory_visible_os",
                bytes(memory.total_bytes),
                "hardware.hardware_reserved_memory_excluded".into(),
            ))
            .child(stat(
                "hardware.currently_available",
                bytes(memory.available_bytes),
                "hardware.varies_according_open_applications".into(),
            ))
            .child(stat(
                "hardware.maximum_declared_capacity",
                memory
                    .max_capacity_bytes
                    .map_or_else(|| not_given("common.not_reported_feminine"), bytes),
                "hardware.declaration_smbios_confirmed_manufacturer".into(),
            ))
            .child(stat(
                "hardware.reported_locations",
                memory.slots.map_or_else(
                    || not_given("common.not_reported_plural"),
                    |s| s.to_string(),
                ),
                tp(
                    "hardware.module_identified_firmware",
                    &[(
                        "v0",
                        if memory.modules.is_empty() {
                            none.as_str().into()
                        } else {
                            (memory.modules.len() as f64).into()
                        },
                    )],
                ),
            ))
            .into_any_element(),
        panel()
            .mb(px(20.))
            .flex()
            .flex_col()
            .child(components::panel_header(
                "hardware.memory_modules",
                Some(tx(&format!(
                    "Source : {}",
                    if memory.source.is_empty() {
                        "hardware.operating_system"
                    } else {
                        &memory.source
                    }
                ))),
                None,
            ))
            .child(modules)
            .into_any_element(),
        two_col(
            half_panel()
                .child(components::panel_header(
                    "hardware.upgrade_in_mind",
                    Some(tx("hardware.check_three_key_points")),
                    None,
                ))
                .child(
                    div()
                        .px(px(23.))
                        .flex()
                        .flex_col()
                        .child(spec_row(
                            tx("hardware.maximum_platform_capacity"),
                            memory
                                .max_capacity_bytes
                                .map_or_else(|| tx("hardware.see_manual"), bytes)
                                .into_any_element(),
                        ))
                        .child(spec_row(
                            tx("hardware.maximum_platform_speed"),
                            tx("hardware.not_communicated_these_interfaces").into_any_element(),
                        ))
                        .child(spec_row(
                            tx("hardware.possible_replacement"),
                            tx(if apple {
                                "hardware.apple_silicon_integrated_memory"
                            } else {
                                "hardware.confirmed_model_manual"
                            })
                            .into_any_element(),
                        )),
                )
                .child(
                    div()
                        .mt(px(7.))
                        .flex()
                        .items_start()
                        .gap(px(10.))
                        .py(px(15.))
                        .px(px(23.))
                        .bg(c((0xf6f9f1, 0x31472a)))
                        .border_t_1()
                        .border_color(c(theme::BORDER))
                        .text_size(px(12.))
                        .line_height(px(19.8))
                        .text_color(c((0x7d9169, 0xadc29a)))
                        .child(icon("book-open", 17., "2", c((0x7d9169, 0xadc29a))).mt(px(2.)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(tx("hardware.motherboard_processor_firmware_together_set")),
                        ),
                ),
            half_panel()
                .child(components::panel_header(
                    "hardware.what_these_data_mean",
                    None,
                    None,
                ))
                .children(memory.notes.iter().map(|n| note(tx(n)).into_any_element())),
        )
        .into_any_element(),
    ]
}

fn storage(app: &Computy) -> Vec<AnyElement> {
    let locale = crate::i18n::locale();
    let inventory = app.inventory.as_ref().unwrap();
    let bytes = |v: f64| intl::bytes(locale, Some(v), 1);
    let tone = if inventory.smart_available {
        NoticeTone::Info
    } else {
        NoticeTone::Warn
    };
    let bold = notice_bold(tone);
    let (lead, rest) = if inventory.smart_available {
        (
            tx("hardware.smartctl_available"),
            tx(" hardware.launch_diagnostic_read_telemetry_accessible"),
        )
    } else {
        (
            tx("hardware.smart_telemetry_not_yet_accessible"),
            tx(" hardware.install_smartmontools_system_update_inventory"),
        )
    };
    let disks = if inventory.disks.is_empty() {
        empty(
            "hard-drive",
            tx("hardware.no_volume_exposed"),
            Some(tx("hardware.system_container_does_not_provide").into_any_element()),
            None,
        )
        .into_any_element()
    } else {
        let cards: Vec<AnyElement> = inventory
            .disks
            .iter()
            .map(|disk| {
                let used = if disk.total_bytes > 0. {
                    1. - disk.available_bytes / disk.total_bytes
                } else {
                    0.
                };
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .gap(px(15.))
                    .p(px(20.))
                    .border_1()
                    .border_color(c(theme::BORDER))
                    .rounded(px(12.))
                    .child(
                        div()
                            .size(px(39.))
                            .flex_none()
                            .rounded(px(10.))
                            .bg(c((0xfaf3e6, 0x403a29)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("hard-drive", 25., "2", c((0xb29a6a, 0xbcad80)))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(spaced(
                                        tx(&disk.mount_point),
                                        16.,
                                        650.,
                                        c(theme::HEADING),
                                        -0.2,
                                        21.,
                                    ))
                                    .child(neutral_badge(tx(match disk.kind.as_str() {
                                        "SSD" => "SSD",
                                        "HDD" => "HDD",
                                        _ => "hardware.type_not_exposed",
                                    }))),
                            )
                            .child(
                                one_line(tx(&disk.name), 11., 400., hex(0xa1ac94))
                                    .line_height(px(18.15))
                                    .mt(px(5.)),
                            )
                            .child(
                                spaced(
                                    bytes(disk.total_bytes),
                                    25.,
                                    550.,
                                    c(theme::TEXT),
                                    -0.6,
                                    33.,
                                )
                                .mt(px(16.)),
                            )
                            .child(
                                div()
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
                                            .bg(hex(0xacbf86)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .child(text(
                                        format!(
                                            "{}{}",
                                            bytes(disk.available_bytes),
                                            tx(" hardware.free_suffix")
                                        ),
                                        11.,
                                        400.,
                                        hex(0xa2ad94),
                                    ))
                                    .child(text(
                                        format!(
                                            "{}{}",
                                            tx(&disk.file_system.to_uppercase()),
                                            tx(if disk.removable { " · Amovible" } else { "" })
                                        ),
                                        11.,
                                        400.,
                                        hex(0xa2ad94),
                                    )),
                            ),
                    )
                    .into_any_element()
            })
            .collect();
        let mut rows = Vec::new();
        let mut iter = cards.into_iter();
        loop {
            let first = iter.next();
            let Some(first) = first else { break };
            let second = iter.next();
            rows.push(
                div()
                    .flex()
                    .gap(px(18.))
                    .child(div().flex_1().min_w_0().flex().child(first))
                    .child(div().flex_1().min_w_0().flex().children(second))
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .px(px(23.))
            .pb(px(23.))
            .children(rows)
            .into_any_element()
    };
    let step = |number: &str, title: &str, body: &str| {
        div()
            .flex()
            .items_start()
            .gap(px(13.))
            .child(
                div()
                    .flex_none()
                    .py(px(6.))
                    .px(px(7.))
                    .bg(c((0xf3f7ed, 0x31472a)))
                    .border_1()
                    .border_color(c((0xe0e9d4, 0x475c34)))
                    .rounded(px(7.))
                    .child(text(tx(number), 10., 400., c((0x9bac85, 0xa3c17f)))),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(spaced(tx(title), 12., 650., c(theme::HEADING), -0.2, 16.))
                    .child(
                        text(tx(body), 12., 400., c((0x7d9169, 0xadc29a)))
                            .line_height(px(22.2))
                            .mt(px(7.)),
                    ),
            )
    };
    vec![
        notice(
            tone,
            vec![rich(&[(&lead, true), (&rest, false)], bold).into_any_element()],
        )
        .text_size(px(12.))
        .line_height(px(21.))
        .mb(px(18.))
        .into_any_element(),
        panel()
            .mb(px(20.))
            .flex()
            .flex_col()
            .child(components::panel_header(
                "common.mounted_volumes",
                Some(tx("hardware.same_physical_disk_may_contain")),
                None,
            ))
            .child(disks)
            .into_any_element(),
        two_col(
            half_panel()
                .child(components::panel_header(
                    "hardware.two_additional_checks",
                    None,
                    None,
                ))
                .child(
                    div()
                        .px(px(23.))
                        .pb(px(22.))
                        .flex()
                        .flex_col()
                        .gap(px(18.))
                        .child(step(
                            "01",
                            "hardware.smart_nvme_indicators",
                            "hardware.reported_state_media_errors_pending",
                        ))
                        .child(step(
                            "02",
                            "hardware.integrity_temporary_file",
                            "hardware.writing_patterns_synchronizing_volume_computy",
                        )),
                ),
            half_panel()
                .child(components::panel_header(
                    "hardware.understanding_scope",
                    None,
                    None,
                ))
                .child(note(tx("hardware.file_control_does_not_cover")))
                .child(note(tx("hardware.caches_usb_bridges_raid_controllers")))
                .child(note(tx("hardware.no_formatting_no_direct_writing"))),
        )
        .into_any_element(),
    ]
}

fn cpu(app: &Computy) -> Vec<AnyElement> {
    let locale = crate::i18n::locale();
    let inventory = app.inventory.as_ref().unwrap();
    let cpu = &inventory.cpu;
    let sensors: Vec<_> = inventory
        .temperatures
        .iter()
        .filter(|s| s.celsius.is_some())
        .collect();
    let sensor_cards: Vec<AnyElement> = sensors
        .iter()
        .map(|sensor| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_start()
                .gap(px(12.))
                .p(px(16.))
                .border_1()
                .border_color(c(theme::BORDER))
                .rounded(px(9.))
                .child(icon("thermometer", 22., "2", hex(0x9dac8d)))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(text(tx(&sensor.name), 12., 400., hex(0x9dac8d)))
                        .child(
                            text(
                                format!(
                                    "{}{}",
                                    intl::number("fr", sensor.celsius.unwrap_or(0.), 1),
                                    tx(" hardware.celsius")
                                ),
                                25.,
                                550.,
                                c(theme::HEADING),
                            )
                            .mt(px(9.)),
                        )
                        .when_some(
                            sensor.critical_celsius.filter(|v| *v != 0.),
                            |d, critical| {
                                d.child(
                                    text(
                                        format!(
                                            "{}{}{}",
                                            tx("hardware.critical_threshold_stated "),
                                            intl::plain(locale, critical),
                                            tx(" hardware.celsius")
                                        ),
                                        10.,
                                        400.,
                                        hex(0xb0b8a4),
                                    )
                                    .mt(px(11.)),
                                )
                            },
                        ),
                )
                .into_any_element()
        })
        .collect();
    let mut rows = Vec::new();
    let mut iter = sensor_cards.into_iter().peekable();
    while iter.peek().is_some() {
        let row: Vec<AnyElement> = iter.by_ref().take(3).collect();
        let missing = 3 - row.len();
        rows.push(
            div()
                .flex()
                .gap(px(16.))
                .children(
                    row.into_iter()
                        .map(|c| div().flex_1().min_w_0().flex().child(c)),
                )
                .children((0..missing).map(|_| div().flex_1()))
                .into_any_element(),
        );
    }
    let frequency = if cpu.frequency_mhz > 0. {
        format!("{} GHz", intl::number(locale, cpu.frequency_mhz / 1000., 2))
    } else {
        tx("common.not_reported_feminine")
    };
    vec![
        panel()
            .flex()
            .items_center()
            .gap(px(28.))
            .p(px(30.))
            .mb(px(20.))
            .child(
                div()
                    .size(px(112.))
                    .flex_none()
                    .rounded(px(23.))
                    .bg(c((0xeeedf5, 0x3a3445)))
                    .border_1()
                    .border_color(c((0xe3deee, 0x4c4359)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("cpu", 68., "1", c((0x9a8eae, 0xb4a4c8)))),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(spaced(
                        tx(if cpu.vendor.is_empty() {
                            "hardware.detected_processor_eyebrow"
                        } else {
                            &cpu.vendor
                        }),
                        10.,
                        650.,
                        hex(0x899582),
                        1.75,
                        16.5,
                    ))
                    .child(
                        spaced(
                            tx(if cpu.name.is_empty() {
                                "hardware.model_not_communicated"
                            } else {
                                &cpu.name
                            }),
                            23.,
                            650.,
                            c(theme::HEADING),
                            -0.7,
                            34.5,
                        )
                        .mt(px(9.)),
                    )
                    .child(
                        text(
                            format!(
                                "{}{}{}{}{}{}",
                                tx(&inventory.arch),
                                tx(" · "),
                                cpu.physical_cores.map_or("—".to_owned(), |v| tn(v as f64)),
                                tx(" hardware.physical_cores_separator "),
                                tn(cpu.logical_cores as f64),
                                tx(" hardware.threads")
                            ),
                            11.,
                            400.,
                            hex(0xa0a895),
                        )
                        .line_height(px(18.15))
                        .mt(px(8.)),
                    ),
            )
            .into_any_element(),
        div()
            .flex()
            .gap(px(16.))
            .mb(px(20.))
            .child(stat(
                "hardware.physical_cores",
                cpu.physical_cores
                    .map_or_else(|| tx("common.not_reported"), |v| v.to_string()),
                "hardware.value_exposed_system".into(),
            ))
            .child(stat(
                "hardware.logic_processors",
                cpu.logical_cores.to_string(),
                "hardware.threads_visible_os".into(),
            ))
            .child(stat(
                "hardware.frequency_reported",
                frequency,
                "hardware.system_measurement_not_maximum_guaranteed".into(),
            ))
            .into_any_element(),
        panel()
            .mb(px(20.))
            .flex()
            .flex_col()
            .child(components::panel_header(
                "hardware.temperature_sensors",
                Some(tx("hardware.one_off_survey_time_updating")),
                None,
            ))
            .child(if rows.is_empty() {
                empty(
                    "thermometer",
                    tx("hardware.temperatures_not_exposed"),
                    Some(tx("hardware.access_depends_os_drivers_hardware").into_any_element()),
                    None,
                )
                .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .px(px(23.))
                    .pb(px(23.))
                    .children(rows)
                    .into_any_element()
            })
            .into_any_element(),
        notice(
            NoticeTone::Info,
            vec![tx("hardware.cpu_test_compares_calculations_parallel").into_any_element()],
        )
        .text_size(px(12.))
        .line_height(px(21.))
        .mb(px(20.))
        .into_any_element(),
    ]
}

pub fn render(app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    let component = app.page.component();
    let (name, description, icon_name) = match app.page {
        Page::Memory => (
            "common.memory",
            "hardware.modules_their_capacity_possibilities_evolution",
            "memory-stick",
        ),
        Page::Storage => (
            "common.storage",
            "hardware.volumes_available_space_disk_indicators",
            "hard-drive",
        ),
        _ => (
            "common.processor",
            "hardware.core_calculations_characteristics_sensors",
            "cpu",
        ),
    };
    let locale = crate::i18n::locale();
    let latest = app
        .reports
        .iter()
        .find(|r| r.checks.iter().any(|c| c.component == component))
        .cloned();
    let summary = component_summary(
        &latest
            .as_ref()
            .map(|l| l.checks.clone())
            .unwrap_or_default(),
        component,
    );
    let body: Vec<AnyElement> = if app.inventory.is_none() {
        vec![
            panel()
                .mb(px(20.))
                .child(empty(
                    icon_name,
                    tx(if app.loading {
                        "hardware.identification_hardware"
                    } else {
                        "hardware.hardware_not_accessible"
                    }),
                    Some(if app.loading {
                        spinner(18., c(theme::MUTED))
                    } else {
                        tx("hardware.connect_rust_engine_read_real").into_any_element()
                    }),
                    None,
                ))
                .into_any_element(),
        ]
    } else {
        let mut items = vec![banner(app, icon_name, cx).into_any_element()];
        items.extend(match app.page {
            Page::Memory => memory(app),
            Page::Storage => storage(app),
            _ => cpu(app),
        });
        items
    };
    let checks: Vec<AnyElement> = latest
        .as_ref()
        .map(|l| {
            l.checks
                .iter()
                .filter(|c| c.component == component)
                .map(|check| check_card(app, check, cx).into_any_element())
                .collect()
        })
        .unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("common.your_hardware_eyebrow"),
            name,
            Some(description),
            Some(
                button(
                    "test-component",
                    ButtonKind::Primary,
                    false,
                    Some("activity"),
                    tx("hardware.test_component"),
                    false,
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.navigate(Page::Diagnostics, window, cx)),
                )
                .into_any_element(),
            ),
        ))
        .children(body)
        .child(
            panel()
                .flex()
                .flex_col()
                .child(components::panel_header(
                    "hardware.latest_controls_recorded",
                    Some(match &latest {
                        Some(l) => tx(&format!(
                            "{} · {}",
                            l.label,
                            intl::date(locale, l.created_at as i64, false)
                        )),
                        None => tx("hardware.results_appear_after_first_diagnostic"),
                    }),
                    latest
                        .as_ref()
                        .map(|_| score(summary.score, summary.status, false).into_any_element()),
                ))
                .child(if latest.is_some() {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .px(px(23.))
                        .pb(px(23.))
                        .children(checks)
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .px(px(23.))
                        .pb(px(23.))
                        .child(icon("activity", 19., "2", c(theme::MUTED)))
                        .child(text(
                            tx("hardware.component_has_not_yet_been"),
                            12.,
                            400.,
                            c(theme::MUTED),
                        ))
                        .child(
                            text_button(
                                "configure",
                                tx("hardware.configure_diagnostic "),
                                Some(("arrow-right", 15.)),
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| this.navigate(Page::Diagnostics, window, cx),
                            )),
                        )
                        .into_any_element()
                }),
        )
        .into_any_element()
}
