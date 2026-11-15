//! Form controls styled like the former stylesheet: select boxes with a
//! dropdown, checkboxes, radio dots and text fields.
use crate::{
    app::Computy,
    i18n::tx,
    theme::{self, c, hex},
    ui::{icon, shadow, text},
};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::{Input, InputState};

pub struct SelectOption {
    pub value: String,
    pub label: String,
}

pub fn option(value: impl ToString, label: impl Into<String>) -> SelectOption {
    SelectOption {
        value: value.to_string(),
        label: label.into(),
    }
}

/// `<select>`: a bordered box; clicking opens an anchored option list.
pub fn select(
    id: &'static str,
    current: &str,
    options: Vec<SelectOption>,
    app: &Computy,
    cx: &mut Context<Computy>,
    on_change: impl Fn(&mut Computy, String, &mut Window, &mut Context<Computy>) + 'static,
) -> Stateful<Div> {
    let open = app
        .pages
        .open_select
        .as_ref()
        .is_some_and(|open| open.as_ref() == id);
    let label = options
        .iter()
        .find(|o| o.value == current)
        .map(|o| o.label.clone())
        .unwrap_or_else(|| current.to_owned());
    let on_change = std::rc::Rc::new(on_change);
    div()
        .id(id)
        .relative()
        .w_full()
        .h(px(40.))
        .flex()
        .items_center()
        .justify_between()
        .px(px(11.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c((0xe2e8d9, 0x425435)))
        .rounded(px(7.))
        .cursor_pointer()
        .child(text(label, 13., 400., c((0x263d34, 0xb1c699))))
        .child(icon("chevron-down", 14., "2", c((0x263d34, 0xb1c699))))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.pages.open_select = if open { None } else { Some(id.into()) };
            cx.notify();
        }))
        .when(open, |d| {
            d.child(
                deferred(
                    anchored().snap_to_window_with_margin(px(8.)).child(
                        div()
                            .id(SharedString::from(format!("{id}-menu")))
                            .mt(px(42.))
                            .min_w(px(180.))
                            .max_h(px(280.))
                            .overflow_y_scroll()
                            .py(px(4.))
                            .bg(c(theme::PANEL))
                            .border_1()
                            .border_color(c((0xe2e8d9, 0x425435)))
                            .rounded(px(7.))
                            .shadow(vec![shadow(0x213b2a, 0.12, 8., 24.)])
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.pages.open_select = None;
                                cx.notify();
                            }))
                            .children(options.into_iter().map(|option| {
                                let selected = option.value == current;
                                let value = option.value.clone();
                                let on_change = on_change.clone();
                                div()
                                    .id(SharedString::from(format!("{id}-{}", option.value)))
                                    .px(px(11.))
                                    .py(px(7.))
                                    .cursor_pointer()
                                    .when(selected, |d| d.bg(c((0xedf3e8, 0x33442a))))
                                    .hover(|s| s.bg(c((0xf7f9f6, 0x253324))))
                                    .child(text(
                                        option.label,
                                        13.,
                                        if selected { 600. } else { 400. },
                                        c((0x263d34, 0xb1c699)),
                                    ))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.pages.open_select = None;
                                        on_change(this, value.clone(), window, cx);
                                        cx.notify();
                                    }))
                            })),
                    ),
                )
                .with_priority(1),
            )
        })
}

/// Labeled select (`.fields-grid > label`).
pub fn field(label: &str, control: impl IntoElement) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(text(tx(label), 12., 400., hex(0x8e9d7c)))
        .child(control)
}

pub fn checkbox_visual(checked: bool) -> impl IntoElement {
    div()
        .size(px(17.))
        .flex_none()
        .rounded(px(4.))
        .border_1()
        .flex()
        .items_center()
        .justify_center()
        .map(|d| {
            if checked {
                d.bg(c((0x78975c, 0x78975c)))
                    .border_color(c((0x78975c, 0x9bbd76)))
            } else {
                d.bg(c((0xffffff, 0x23311f)))
                    .border_color(c((0xc9d5bb, 0x6d8954)))
            }
        })
        .when(checked, |d| d.child(icon("check", 13., "2", hex(0xffffff))))
}

pub fn radio_visual(checked: bool) -> impl IntoElement {
    div()
        .size(px(13.))
        .flex_none()
        .rounded_full()
        .border_1()
        .bg(c((0xffffff, 0x23311f)))
        .border_color(if checked {
            c((0x9bb47d, 0x6d8954))
        } else {
            c((0xd6dfca, 0x6d8954))
        })
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |d| {
            d.child(
                div()
                    .size(px(5.))
                    .rounded_full()
                    .bg(c((0x7f9c5e, 0xb7d68d))),
            )
        })
}

/// `.text-input` around a GPUI input state.
pub fn text_input(state: &Entity<InputState>) -> impl IntoElement {
    div()
        .w_full()
        .h(px(42.))
        .px(px(14.))
        .flex()
        .items_center()
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c((0xe0e6d8, 0x425435)))
        .rounded(px(7.))
        .text_size(px(13.))
        .text_color(c((0x263d34, 0xb1c699)))
        .child(Input::new(state).appearance(false).w_full())
}
