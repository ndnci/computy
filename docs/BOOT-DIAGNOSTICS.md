# Preboot memory testing

Computy distinguishes **in-session checks** (quick, standard and in-depth profiles, without rebooting) from **preboot testing**. The latter is a phase of the same product, included in the same installer; it does not require a separate Memtest86+ installation.

The integrated component derives from Memtest86+ 9.00. Test algorithms remain unchanged; its presentation is adapted to Computy. The standalone component is GPLv2, with notices and corresponding sources included in the package. The [component record](../third_party/computy-memory-boot/COMPUTY.md) documents its provenance.

## Linux without a USB drive

The wizard checks x86-64 architecture, firmware, Secure Boot, the installed component and GRUB. After confirmation in the application and PolicyKit authorization, a privileged Rust helper selects only the `computy-memory-test` entry for the next boot using `grub-reboot`. It verifies the selection before enabling the reboot button.

The default operating system remains unchanged. GRUB must be able to clear `next_entry` after use; this workflow is restricted to supported boot environments without LVM/RAID/encryption abstractions and with GRUB's `ext2` filesystem support (the ext family). Unsupported configurations are rejected. An existing one-time selection is never overwritten.

Automatic first-time entry creation is refused when a custom GRUB default could be affected by entry positions. An existing Computy entry is preserved during upgrades.

Preparation can be cancelled. Rebooting requires separate consent. Development tests never schedule a boot on the host machine.

**Secure Boot:** the modified Computy binary is not signed by an authority trusted by the firmware. Enabled or indeterminate Secure Boot blocks this launch. Computy does not change UEFI keys or disable the protection. Forking or rewriting the engine in Rust would not remove this constraint. In-session testing remains available.

## Linux with a USB drive

The wizard lists **mounted FAT32 USB volumes**. It excludes disks containing system mount points, asks the user to select a drive, and requires confirmation of the destination. The identifier includes the UUID to distinguish unplugged and reconnected drives.

Computy copies the engine to `EFI/BOOT/BOOTX64.EFI` without formatting the drive or replacing an existing bootloader. A drive already containing that file is rejected. Other filesystems must be prepared separately with the operating system's disk utility; this version does not erase or repartition devices.

Choosing the drive at startup depends on the firmware; Computy explains this step. Secure Boot still applies. The engine displays progress, passes and errors; Escape exits and reboots. F6 can save a report to a FAT32 volume supported by the engine. Avoid connecting multiple USB drives simultaneously to keep the report destination unambiguous.

## Results and limitations

Returning to Linux **does not prove** that testing succeeded or completed. No score is invented. This version cannot automatically recover standalone results without an available persistent channel; the wizard explains this before preparation. Record the result on screen or save the USB report.

On Windows, Computy opens the system memory diagnostic and imports a recent result event after returning. Reopening Computy at login is checked by default in that workflow and can be disabled. The application's general startup preference remains explicit. On Mac, the Apple workflow depends on the model and is not automated here.

## Validation

The engine actually booted in two QEMU VMs, BIOS and UEFI, without access to the PC's disks. Tests 0, 1 and 2 ran for boot validation; this does not certify all algorithms or physical hardware. Screenshots and the manifest are in `docs/screenshots/boot-*`.

Rust tests cover rejection of Secure Boot, unsupported GRUB configurations, another boot selection, unconfirmed requests, and USB drives containing a system or an existing loader. UI tests verify separate consent steps. Real GRUB mutation has not been executed on the host machine.

Since 0.1.4, the generator permits `grub-mkconfig_lib` to initialize optional variables without `set -u` and prepares device setup before emitting an entry. Package tests use real Debian and Ubuntu libraries with fixture device probes in a container. They also validate GRUB syntax and ensure failures produce no partial entry. These checks never schedule a boot.

Sources: [Memtest86+ booting and Secure Boot](https://memtest.org/readme), [GRUB one-time next_entry](https://www.gnu.org/software/grub/manual/grub/html_node/next_005fentry.html), [component license](../third_party/computy-memory-boot/LICENSE).
