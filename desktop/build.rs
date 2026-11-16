//! Embeds generated icons (`assets/icons/<name>@<stroke>.svg`) and decorative
//! art (`assets/art/*.png`) in a sorted table.
use std::{env, fs, path::Path};

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut entries = Vec::new();
    for (directory, extension) in [("icons", "svg"), ("art", "png")] {
        println!("cargo:rerun-if-changed=assets/{directory}");
        for entry in fs::read_dir(Path::new("assets").join(directory)).expect(directory) {
            let name = entry.unwrap().file_name().into_string().unwrap();
            if name.ends_with(extension) {
                entries.push(format!("{directory}/{name}"));
            }
        }
    }
    entries.sort();
    let mut code = String::from("pub const ICONS: &[(&str, &[u8])] = &[\n");
    for name in &entries {
        // Debug formatting escapes Windows backslashes in the generated literal.
        let path = Path::new(&root).join("assets").join(name);
        code.push_str(&format!("    ({name:?}, include_bytes!({path:?})),\n"));
    }
    code.push_str("];\n");
    code.push_str(
        "fn icon(path: &str) -> Option<&'static [u8]> {\n    ICONS.binary_search_by(|(name, _)| (*name).cmp(path)).ok().map(|i| ICONS[i].1)\n}\n",
    );
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("icons.rs"),
        code,
    )
    .unwrap();
}
