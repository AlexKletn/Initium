# Initium

[Русская версия](README.ru.md)

[Contributing](CONTRIBUTING.md) · [GPL-2.0-only](LICENSE)

Rust workspace for the early boot component.

## Arch Linux package

Build from a source archive generated from the Initium checkout.

```bash
tar --transform='s,^,initium-0.1.0/,' -czf initium-0.1.0.tar.gz \
    .cargo Cargo.toml Cargo.lock crates systemd LICENSE \
    README{,.ru}.md CONTRIBUTING{,.ru}.md CODE_QUALITY{,.ru}.md COMMIT_CONVENTIONS{,.ru}.md
makepkg -si
```

The package installs
`initium`, `initium-gui-preview`, `initium-shutdown`, the shutdown units,
and the documentation. Configure and enable the units for
your device as described below. The package leaves the initramfs and the
device's boot and shutdown configuration to the system image. Include the
GUI binary and its shared libraries when assembling an initramfs.

## Workspace crates

- [`initium`](crates/initium/)
- [`initium-config`](crates/initium-config/)
- [`initium-flow`](crates/initium-flow/)
- [`initium-gui`](crates/initium-gui/)

## GUI build

`initium-gui` builds the bundled LVGL C sources and needs a C compiler and
libclang at build time. For cross-compilation, use a C compiler for the target.

## Display

Call `initium_gui::initialize()` before connecting a display through
`DisplayPort`. Use `Fbdev::new("/dev/fb0")` for a Linux framebuffer or
`DrmKms::new("/dev/dri/card0", None)` for DRM/KMS. Pass a connector ID in
place of `None` to select a specific output. Keep the returned
`DisplayConnection` alive for as long as the GUI uses the display.

The adapters use LVGL's Linux drivers. Install the libdrm development
headers and library to build them. If the headers are outside the system
include directories, set `C_INCLUDE_PATH` to include both the directory
with `xf86drm.h` and the one with `drm.h`. For a static musl build, supply
libdrm headers and `libdrm.a` for the target and pass the archive's
directory to Rust via `-L native=<directory>`.

To choose the adapter in TOML, add the optional `[ui.display]` section and
pass its settings to `initium_gui::connect_display` after initialization.
`backend = "fbdev"` takes a framebuffer path. `backend = "drm-kms"`
takes a card path and an optional `connector_id`.

```toml
[ui.display]
backend = "drm-kms"
path = "/dev/dri/card0"
```

## Splash configuration

The splash configuration specifies the brand label and the RLE-compressed
BGRA sequences for the intro and loop. At startup the GUI must read the
frame left by the ABL bootloader directly from the framebuffer before its
first update. The GUI uses those pixels as the base and applies the cover
rectangle. The frame remains in memory. Call `BootloaderFrame::capture`
with the Linux framebuffer device before connecting the display, then
use `DisplayConnection::show_bootloader_frame` to present that frame.

The presence of `[ui.splash.bootloader_frame]` enables framebuffer capture.
The optional cover uses pixel coordinates from the top left at the reference
resolution and an RGB triplet. The GUI scales the rectangle to the actual
frame size.

```toml
[ui.splash]
brand_text = "Example Linux"
intro_frames = "/assets/intro.frames"
loop_frames = "/assets/loop.frames"

[ui.splash.bootloader_frame.cover]
reference_width = 1920
reference_height = 1200
x = 760
y = 930
width = 400
height = 96
rgb = [0, 0, 0]
```

When a platform exposes the ABL framebuffer as a Device Tree reserved-memory
region, add `[ui.splash.bootloader_frame.reserved_memory]`. The node is
resolved under `reserved-memory` in the running Device Tree. Initium reads
the packed XRGB8888 pixels from that region through `/dev/mem`. When this
section is absent, capture uses the framebuffer device passed by the caller.

```toml
[ui.splash.bootloader_frame.reserved_memory]
node = "memory@88000000"
width = 1920
height = 1200
```

## ABL frame preview

Build `cargo build -p initium-gui --bin initium-gui-preview` for the target.
In early boot, run `initium-gui-preview <config.toml>` once the configured DRM
or framebuffer device is available. The preview captures the frame before
opening the display, applies the cover, and displays it for five seconds.

Set `capture_device` under `[ui.splash.bootloader_frame]` to choose the
framebuffer for capture. Without a reserved-memory source or an explicit
capture device, DRM uses `/dev/fb0`. Intro and loop `.frames` files can be loaded
from initramfs. `[ui.splash.animation]` defines frame dimensions and frame
rates for the RLE streams. Use `--hold` to keep the DRM display and the
loop active after handoff.

The optional outro table controls the merge and collapse durations,
the radii in frame pixels, and the RGB weights used to locate the moving
part of the image. Omit the table to skip the outro.

```toml
[ui.splash.animation.outro]
merge_ms = 360
collapse_ms = 240
core_radius_px = 80
region_radius_px = 150
satellite_rgb_weights = [0, 1, 0]
```

## Menu configuration

Set `ui.touch_path` to the touchscreen's evdev node to enable the interactive
menu. Double-tap the center of the animated logo to open or close it. The
splash rises before the central menu tiles appear.

- The GUI parses and displays the boot log.
- The hardware panel displays devices according to their configuration
  descriptions.
- The power menu is an ordered list of entries. Each entry specifies an
  icon, a label, and a command.
- The userspace selector is optional. It lists the configured names and icons in order. An empty or omitted list hides the selector.

The built-in boot log panel reads `/dev/kmsg` when opened. It shows up to
192 recent records with their timestamp and severity and opens at the
latest entry. Use `DisplayConnection::show_boot_log` to open it.

The preview can read boot state from `ui.boot_state_dir`. The userspace
selector locks when the root is ready and the final boot pause is closed,
or as soon as the root handoff stage is observed. It stays hidden when
the menu is reopened.

```toml
[[ui.userspace]]
name = "Linux"
icon = "/assets/linux.png"

[[ui.userspace]]
name = "Recovery"
icon = "/assets/recovery.png"
```

Userspace icons are PNG files. Pass `config.ui.userspace` to
`DisplayConnection::show_userspace_selector` and keep the returned selector
alive while it is visible. Call `DisplayConnection::refresh` to present it.

Use `DisplayConnection::show_power_menu` to display the configured entries.
When LVGL reports a click, the caller can run the queued command with
`PowerMenu::run_pending_command`. Arguments are passed directly to the program.

```toml
[[ui.power_menu]]
name = "Reboot"
icon = "/assets/reboot.png"
command = { program = "/usr/bin/systemctl", args = ["reboot"] }
```

## Boot scenario

`initium <config.toml>` validates the complete scenario and runs the steps
in order. Each step has a unique `id` and a registered operation `kind`.
A failed step stops the scenario. The built-in `command` operation runs a
program with its argument list.

The `mount` operation mounts a filesystem directly. The target directory
must already exist. Set `options` for filesystem-specific options and
`flags` for mount flags such as `read-only`.

```toml
[[steps]]
id = "mount-proc"
kind = "mount"
source = "proc"
target = "/proc"
filesystem = "proc"
flags = ["no-suid", "no-dev", "no-exec"]
```

Each `[[ui.hardware]]` entry describes a device and its readiness probe.
The optional `detected` probe distinguishes detected devices from pending ones.
A probe checks whether a path exists, compares its trimmed text to
`equals`, or checks whether a `glob` matches a live sysfs entry. Devices can
omit `ready` when only detection is available. `HardwarePanel::update` rereads the configured probes.

```toml
[[ui.hardware]]
name = "Battery"
ready = { path = "/sys/class/power_supply/mm8013/present", equals = "1" }
```

## Shared theme

The boot UI and shutdown splash both read `/etc/initium/theme.toml`.
Install the same file in the initramfs and the installed system.

```toml
background_rgb = 0x000000
accent_rgb = 0xf0c16a
```

These values are also the defaults when the file is absent. The theme
colors the boot background after the ABL handoff, the boot label and
menu accents, and the shutdown background and captions. Animation
frames retain their own colors.

## Shutdown splash

Build `cargo build --release -p initium-gui --bin initium-shutdown` and
install the binary at `/usr/lib/initium/initium-shutdown`. Install the units
from [systemd/](systemd/) and the device configuration at
`/etc/initium/shutdown.toml`. Enable `initium-shutdown@poweroff.service` and
`initium-shutdown@reboot.service`. Enable `initium-shutdown-blank.service`
when the device configuration specifies a backlight brightness path.

Initium waits up to two seconds for graphical sessions and DRM master on
shutdown. It reuses the active DRM mode and can fall back to a configured
mode when the output is idle. It keeps the rendered frame on screen and
optionally mirrors it to fbcon. The splash service has a three-second
timeout. The backlight unit runs after `final.target`.

```toml
[shutdown]
drm_device = "/dev/dri/card0"
console_framebuffer = "/dev/fb0"
console_blank = "/sys/class/graphics/fb0/blank"
# poweroff_caption = "/usr/share/initium/theme/poweroff.bgra"
# reboot_caption = "/usr/share/initium/theme/reboot.bgra"
```

Without custom images, Initium draws a text caption. Configure
`poweroff_caption` and `reboot_caption` for raw BGRA images with straight
alpha and declare their dimensions in `caption_size = [width, height]`.
Initium applies the theme accent to their silhouettes. Set `connector_id_file` to the
DRM connector ID attribute to select a specific output. Without it,
Initium selects a connected output on the configured card, preferring
one with an active mode. Set `seat` to select the login seat (default `seat0`).
`fallback_size` and `fallback_refresh_hz` select the idle-output mode.
