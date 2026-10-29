//! StatusNotifier items with real activation events and persistent identities.
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use zbus::blocking::{Connection, connection::Builder};
pub type Activation = Arc<dyn Fn(String, String, String, i32, i32) + Send + Sync>;
type IconPixmaps = Vec<(i32, i32, Vec<u8>)>;
type ToolTip = (String, IconPixmaps, String, String);
#[derive(Clone, Default)]
pub struct ItemData {
    pub id: String,
    pub metric: String,
    pub volume: String,
    pub title: String,
    pub label: String,
    pub active: bool,
    pub pixels: Vec<u8>,
    pub menu: Vec<crate::linux_menu::Row>,
    pub revision: u32,
}
struct Item {
    data: Arc<RwLock<ItemData>>,
    activate: Activation,
    menu_path: zbus::zvariant::OwnedObjectPath,
}
#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn category(&self) -> &str {
        "Hardware"
    }
    #[zbus(property)]
    fn id(&self) -> String {
        self.data
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .id
            .clone()
    }
    #[zbus(property)]
    fn title(&self) -> String {
        self.data
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .title
            .clone()
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        if self.data.read().unwrap_or_else(|e| e.into_inner()).active {
            "Active"
        } else {
            "Passive"
        }
    }
    #[zbus(property)]
    fn window_id(&self) -> u32 {
        0
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        let data = self.data.read().unwrap_or_else(|e| e.into_inner());
        vec![(
            64,
            64,
            data.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|rgba| [rgba[3], rgba[0], rgba[1], rgba[2]])
                .collect(),
        )]
    }
    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        vec![]
    }
    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        vec![]
    }
    #[zbus(property)]
    fn attention_movie_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip {
        (String::new(), vec![], self.title(), String::new())
    }
    #[zbus(property, name = "XAyatanaLabel")]
    fn ayatana_label(&self) -> String {
        self.data
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .label
            .clone()
    }
    #[zbus(property, name = "XAyatanaLabelGuide")]
    fn ayatana_guide(&self) -> &str {
        "100%"
    }
    #[zbus(property, name = "XAyatanaOrderingIndex")]
    fn ayatana_order(&self) -> u32 {
        0
    }
    #[zbus(property)]
    fn menu(&self) -> zbus::zvariant::OwnedObjectPath {
        self.menu_path.clone()
    }
    fn activate(&self, x: i32, y: i32) {
        let data = self.data.read().unwrap_or_else(|e| e.into_inner()).clone();
        if data.active {
            (self.activate)("panel".into(), data.metric, data.volume, x, y);
        }
    }
    fn secondary_activate(&self, x: i32, y: i32) {
        self.activate(x, y);
    }
    fn context_menu(&self, x: i32, y: i32) {
        self.activate(x, y);
    }
    fn scroll(&self, _delta: i32, _orientation: &str) {}
}
struct Registration {
    connection: Connection,
    service: String,
    path: String,
    data: Arc<RwLock<ItemData>>,
    registered_owner: String,
}
pub struct LinuxIndicators {
    connection: Connection,
    items: BTreeMap<String, Registration>,
    next: u64,
    activate: Activation,
}
impl LinuxIndicators {
    pub fn new(activate: Activation) -> Result<Self, String> {
        let connection = Builder::session()
            .map_err(|e| e.to_string())?
            .method_timeout(Duration::from_millis(1200))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection,
            items: BTreeMap::new(),
            next: 0,
            activate,
        })
    }
    pub fn sync(&mut self, values: Vec<ItemData>) -> Result<usize, String> {
        let owner = self
            .connection
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetNameOwner",
                &("org.kde.StatusNotifierWatcher",),
            )
            .ok()
            .and_then(|r| r.body().deserialize::<String>().ok())
            .unwrap_or_default();
        let registered: Option<Vec<String>> = self
            .connection
            .call_method(
                Some("org.kde.StatusNotifierWatcher"),
                "/StatusNotifierWatcher",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(
                    "org.kde.StatusNotifierWatcher",
                    "RegisteredStatusNotifierItems",
                ),
            )
            .ok()
            .and_then(|reply| {
                reply
                    .body()
                    .deserialize::<zbus::zvariant::OwnedValue>()
                    .ok()
            })
            .and_then(|value| Vec::<String>::try_from(value).ok());
        for (id, registration) in &self.items {
            if !values.iter().any(|v| &v.id == id) {
                let mut data = registration.data.write().unwrap_or_else(|e| e.into_inner());
                if data.active {
                    data.active = false;
                    drop(data);
                    Self::signal(
                        &registration.connection,
                        &registration.path,
                        "NewStatus",
                        &("Passive",),
                    );
                }
            }
        }
        for mut value in values {
            let id = value.id.clone();
            if !self.items.contains_key(&id) {
                // One persistent bus identity per indicator also survives GNOME's
                // service-based discovery, which coalesces objects sharing a service.
                let service = format!(
                    "org.kde.StatusNotifierItem-{}-Computy-{}",
                    std::process::id(),
                    self.next
                );
                self.next += 1;
                let path = "/StatusNotifierItem".to_owned();
                let connection = Builder::session()
                    .map_err(|e| e.to_string())?
                    .method_timeout(Duration::from_millis(1200))
                    .build()
                    .map_err(|e| e.to_string())?;
                let data = Arc::new(RwLock::new(value.clone()));
                connection
                    .object_server()
                    .at(
                        path.as_str(),
                        Item {
                            data: data.clone(),
                            activate: self.activate.clone(),
                            menu_path: format!("{path}/Menu").try_into().unwrap(),
                        },
                    )
                    .map_err(|e| e.to_string())?;
                connection
                    .object_server()
                    .at(
                        format!("{path}/Menu"),
                        crate::linux_menu::Menu {
                            data: data.clone(),
                            activate: self.activate.clone(),
                        },
                    )
                    .map_err(|e| e.to_string())?;
                connection
                    .request_name(service.as_str())
                    .map_err(|e| e.to_string())?;
                self.items.insert(
                    id.clone(),
                    Registration {
                        connection,
                        service,
                        path,
                        data,
                        registered_owner: String::new(),
                    },
                );
            }
            let registration = self.items.get_mut(&id).unwrap();
            let before = registration
                .data
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            value.revision = before.revision + u32::from(before.menu.len() != value.menu.len());
            *registration.data.write().unwrap_or_else(|e| e.into_inner()) = value.clone();
            let path = registration.path.clone();
            let unique = registration
                .connection
                .unique_name()
                .map(|name| name.as_str())
                .unwrap_or("");
            if registered.as_ref().is_some_and(|items| {
                !items.iter().any(|item| {
                    item == &registration.service
                        || item == &format!("{}{path}", registration.service)
                        || item == &format!("{unique}{path}")
                        || item == &format!("{unique}@{path}")
                })
            }) {
                registration.registered_owner.clear();
            }
            if !owner.is_empty()
                && (registration.registered_owner != owner || !before.active)
                && registration
                    .connection
                    .call_method(
                        Some("org.kde.StatusNotifierWatcher"),
                        "/StatusNotifierWatcher",
                        Some("org.kde.StatusNotifierWatcher"),
                        "RegisterStatusNotifierItem",
                        &(registration.service.as_str(),),
                    )
                    .is_ok()
            {
                registration.registered_owner = owner.clone();
            }
            if before.active != value.active {
                Self::signal(
                    &registration.connection,
                    &path,
                    "NewStatus",
                    &(if value.active { "Active" } else { "Passive" },),
                );
            }
            if before.pixels != value.pixels {
                Self::signal(&registration.connection, &path, "NewIcon", &());
            }
            if before.title != value.title {
                Self::signal(&registration.connection, &path, "NewTitle", &());
                Self::signal(&registration.connection, &path, "NewToolTip", &());
            }
            if before.menu.len() != value.menu.len() {
                let _ = registration.connection.emit_signal(
                    None::<&str>,
                    format!("{path}/Menu"),
                    "com.canonical.dbusmenu",
                    "LayoutUpdated",
                    &(value.revision, 0i32),
                );
            } else if before.menu != value.menu {
                let changed: Vec<_> = value.menu.iter().map(|r| (r.id, r.properties())).collect();
                let removed: Vec<(i32, Vec<String>)> = vec![];
                let _ = registration.connection.emit_signal(
                    None::<&str>,
                    format!("{path}/Menu"),
                    "com.canonical.dbusmenu",
                    "ItemsPropertiesUpdated",
                    &(changed, removed),
                );
            }
            if before.label != value.label {
                Self::signal(
                    &registration.connection,
                    &path,
                    "XAyatanaNewLabel",
                    &(value.label.as_str(), "100%"),
                );
            }
        }
        Ok(if owner.is_empty() {
            0
        } else {
            self.items
                .values()
                .filter(|i| {
                    i.registered_owner == owner
                        && i.data.read().unwrap_or_else(|e| e.into_inner()).active
                })
                .count()
        })
    }
    fn signal<T: serde::Serialize + zbus::zvariant::DynamicType>(
        connection: &Connection,
        path: &str,
        name: &str,
        body: &T,
    ) {
        let _ =
            connection.emit_signal(None::<&str>, path, "org.kde.StatusNotifierItem", name, body);
    }
    pub fn entries(&self) -> Vec<serde_json::Value> {
        self.items.iter().map(|(id,item)|serde_json::json!({"id":id,"path":item.path,"bus":item.connection.unique_name().map(|n|n.as_str()),"service":item.service,"active":item.data.read().unwrap_or_else(|e|e.into_inner()).active})).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Watcher(Arc<std::sync::Mutex<Vec<String>>>);
    #[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
    impl Watcher {
        fn register_status_notifier_item(&self, service: String) {
            self.0.lock().unwrap().push(service);
        }
    }
    #[test]
    #[ignore = "requires a private D-Bus session"]
    fn repeated_disable_enable_keeps_exported_identity_and_real_activation() {
        assert!(std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some());
        let registrations = Arc::new(std::sync::Mutex::new(vec![]));
        let watcher = Connection::session().unwrap();
        watcher
            .request_name("org.kde.StatusNotifierWatcher")
            .unwrap();
        watcher
            .object_server()
            .at("/StatusNotifierWatcher", Watcher(registrations.clone()))
            .unwrap();
        let activated = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = activated.clone();
        let mut client = LinuxIndicators::new(Arc::new(move |_, metric, _, _, _| {
            assert_eq!(metric, "memory");
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }))
        .unwrap();
        let item = ItemData {
            id: "computy-memory".into(),
            metric: "memory".into(),
            active: true,
            pixels: vec![255; 64 * 64 * 4],
            ..Default::default()
        };
        for _ in 0..3 {
            assert_eq!(client.sync(vec![item.clone()]).unwrap(), 1);
            let before = client.entries()[0]["path"].as_str().unwrap().to_owned();
            assert_eq!(client.sync(vec![]).unwrap(), 0);
            assert_eq!(client.sync(vec![item.clone()]).unwrap(), 1);
            assert_eq!(client.entries()[0]["path"], before);
        }
        let path = client.entries()[0]["path"].as_str().unwrap().to_owned();
        let proxy = zbus::blocking::Proxy::new(
            &watcher,
            client
                .items
                .values()
                .next()
                .unwrap()
                .connection
                .unique_name()
                .unwrap()
                .as_str(),
            path.as_str(),
            "org.kde.StatusNotifierItem",
        )
        .unwrap();
        assert_eq!(proxy.get_property::<String>("Status").unwrap(), "Active");
        assert!(!proxy.get_property::<bool>("ItemIsMenu").unwrap());
        let _: () = proxy.call("Activate", &(100i32, 24i32)).unwrap();
        assert!(activated.load(std::sync::atomic::Ordering::SeqCst));
        assert!(registrations.lock().unwrap().len() >= 3);
    }
}
