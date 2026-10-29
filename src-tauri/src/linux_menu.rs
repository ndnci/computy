//! Standard DBusMenu fallback for hosts which reserve activation for double-click.
use crate::linux_indicator::{Activation, ItemData};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use zbus::zvariant::Value;
#[derive(Clone, Default, PartialEq)]
pub struct Row {
    pub id: i32,
    pub label: String,
    pub icon: String,
    pub png: Vec<u8>,
    pub action: String,
    pub separator: bool,
}
impl Row {
    pub fn properties(&self) -> HashMap<String, Value<'static>> {
        let mut props = HashMap::from([
            ("label".into(), Value::new(self.label.clone())),
            ("enabled".into(), Value::new(!self.action.is_empty())),
            ("visible".into(), Value::new(true)),
        ]);
        if self.separator {
            props.insert("type".into(), Value::new("separator"));
        }
        if !self.icon.is_empty() {
            props.insert("icon-name".into(), Value::new(self.icon.clone()));
        }
        if !self.png.is_empty() {
            props.insert("icon-data".into(), Value::new(self.png.clone()));
        }
        props
    }
}
type Layout = (i32, HashMap<String, Value<'static>>, Vec<Value<'static>>);
pub struct Menu {
    pub data: Arc<RwLock<ItemData>>,
    pub activate: Activation,
}
#[zbus::interface(name = "com.canonical.dbusmenu")]
impl Menu {
    #[zbus(property)]
    fn version(&self) -> u32 {
        4
    }
    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        vec![]
    }
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (u32, Layout) {
        let data = self.data.read().unwrap_or_else(|e| e.into_inner());
        if parent_id != 0 {
            let props = data
                .menu
                .iter()
                .find(|r| r.id == parent_id)
                .map(Row::properties)
                .unwrap_or_default();
            return (data.revision, (parent_id, props, vec![]));
        }
        let children = if recursion_depth == 0 {
            vec![]
        } else {
            data.menu
                .iter()
                .map(|row| Value::new((row.id, row.properties(), Vec::<Value<'static>>::new())))
                .collect()
        };
        (
            data.revision,
            (
                0,
                HashMap::from([("children-display".into(), Value::new("submenu"))]),
                children,
            ),
        )
    }
    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, Value<'static>>)> {
        self.data
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .menu
            .iter()
            .filter(|r| ids.is_empty() || ids.contains(&r.id))
            .map(|r| (r.id, r.properties()))
            .collect()
    }
    fn event(&self, id: i32, event_id: &str, _data: Value<'_>, _timestamp: u32) {
        if event_id != "clicked" {
            return;
        }
        let data = self.data.read().unwrap_or_else(|e| e.into_inner()).clone();
        if data.active
            && let Some(row) = data.menu.iter().find(|r| r.id == id)
            && !row.action.is_empty()
        {
            (self.activate)(row.action.clone(), data.metric, data.volume, -1, -1);
        }
    }
    fn event_group(&self, events: Vec<(i32, String, Value<'_>, u32)>) -> Vec<i32> {
        for (id, event, data, time) in events {
            self.event(id, &event, data, time);
        }
        vec![]
    }
    fn about_to_show(&self, _id: i32) -> bool {
        false
    }
    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (vec![], vec![])
    }
}
