use computy_engine::usb_boot::{copy_payload, parse_targets};
use serde_json::json;
#[test]
fn fat16_is_not_presented_as_a_fat32_report_target() {
    let value = json!({"blockdevices":[{"type":"disk","tran":"usb","children":[{"name":"/dev/sdb1","fstype":"vfat","fsver":"FAT16","uuid":"FAT16","mountpoints":["/media/usb"]}]}]});
    assert!(parse_targets(&value).is_empty());
}
#[test]
fn only_mounted_fat_usb_volumes_are_candidates_and_the_os_disk_is_excluded() {
    let value = json!({"blockdevices":[
        {"type":"disk","tran":"usb","children":[{"name":"/dev/sdb1","fstype":"vfat","fsver":"FAT32","uuid":"ABCD","mountpoints":["/media/test/usb"],"size":100,"label":"USB"}]},
        {"type":"disk","tran":"usb","children":[{"name":"/dev/sdc1","fstype":"ext4","mountpoints":["/"]},{"name":"/dev/sdc2","fstype":"vfat","fsver":"FAT32","uuid":"OSDISK","mountpoints":["/media/os"]}]},
        {"type":"disk","tran":"sata","children":[{"name":"/dev/sda1","fstype":"vfat","fsver":"FAT32","uuid":"INTERNAL","mountpoints":["/media/internal"]}]}
    ]});
    let targets = parse_targets(&value);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, "/dev/sdb1:ABCD");
}
#[test]
fn preparation_preserves_existing_files_and_refuses_to_overwrite_a_bootloader() {
    let root =
        std::env::temp_dir().join(format!("computy-usb-{}", computy_engine::domain::new_id()));
    std::fs::create_dir_all(&root).unwrap();
    let payload = root.join("payload.efi");
    std::fs::write(&payload, b"payload fixture").unwrap();
    let mount = root.join("usb");
    std::fs::create_dir(&mount).unwrap();
    // macOS temp folders and Windows extended paths can have canonical aliases.
    let mount = mount.canonicalize().unwrap();
    std::fs::write(mount.join("personal.txt"), b"keep").unwrap();
    copy_payload(&payload, &mount).unwrap();
    assert_eq!(
        std::fs::read(mount.join("EFI/BOOT/BOOTX64.EFI")).unwrap(),
        b"payload fixture"
    );
    assert!(copy_payload(&payload, &mount).is_err());
    assert_eq!(std::fs::read(mount.join("personal.txt")).unwrap(), b"keep");
    std::fs::remove_dir_all(root).unwrap();
}
#[cfg(unix)]
#[test]
fn a_usb_directory_symlink_cannot_redirect_writes() {
    let root = std::env::temp_dir().join(format!(
        "computy-usb-link-{}",
        computy_engine::domain::new_id()
    ));
    std::fs::create_dir_all(root.join("usb")).unwrap();
    let root = root.canonicalize().unwrap();
    std::os::unix::fs::symlink(&root, root.join("usb/EFI")).unwrap();
    assert!(copy_payload(&root.join("missing.efi"), &root.join("usb")).is_err());
    assert!(!root.join("BOOT").exists());
    std::fs::remove_dir_all(root).unwrap();
}
