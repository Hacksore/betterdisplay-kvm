# BetterDisplay KVM

A Rust-based KVM switch utility for BetterDisplay that utilizes the [`betterdisplaycli`](https://github.com/waydabber/betterdisplaycli).

## Bill of Materials (BOM)

- Mac
- [USB peripheral switch](https://a.co/d/dRZjOcX)
- Monitor supporting [DDC/CI](https://en.wikipedia.org/wiki/Display_Data_Channel) 

## How it works

This works by using the BetterDisplay app and CLI to issue commands to your monitor when a configured USB device is connected or disconnected via the `betterdisplay-kvm` Rust program. It uses the [DDC/CI](https://en.wikipedia.org/wiki/Display_Data_Channel) protocol to send commands directly to your monitor. 

With a single press of a button, you can switch to your gaming PC or MacBook seamlessly.

## Diagram

![diagram](./betterdisplay-kvm-diagram.png)

## Why not use a KVM?

Because they don’t support high refresh rates without spending an ungodly amount of money.

## Config

This config lives in `~/.config/betterdisplay-kvm/config.toml`.

```toml
# the USB device you'd like to watch for
usb_device_id = "046d:c547"
# ID that betterdisplaycli uses to configure input
system_one_input = 15
# ID that betterdisplaycli uses to configure input
system_two_input = 18
# log level
log_level = "debug"
# if you use an LG monitor that doesn't follow the spec, this might work if you enable it
ddc_alt = false
```

## Finding `usb_device_id`

Run the binary with `--list`. It prints one line per connected USB device to stdout, tab-separated:

1. **`usb_device_id`** — vendor and product in lowercase hex (`vvvv:pppp`), same format as in `config.toml`
2. Manufacturer
3. Product name
4. Serial number (empty if the device does not expose one)

```bash
betterdisplay-kvm --list
```

Example line:

```text
046d:c54d	Logitech	USB Receiver	3480336C3135
```

Copy the first field into `usb_device_id`. The daemon matches on vendor and product only, not the serial; the serial column is there so you can tell devices apart when several share the same id.

## Development

```bash
RUST_LOG=debug cargo watch -x "run -- --launch"
```

## Install

From crates.io:

```bash
cargo install betterdisplay-kvm
betterdisplay-kvm --install
```

For local development, run `./install.sh`. It builds the release binary, installs
it under `~/Library/Application Support/betterdisplay-kvm`, and starts the
LaunchAgent.

The `--install` command copies the running binary into
`~/Library/Application Support/betterdisplay-kvm/betterdisplay-kvm`, writes the
LaunchAgent plist, and refreshes launchd so the loaded job uses the newly
installed binary.

## Upgrade

After publishing a new version to crates.io, users can upgrade with:

```bash
cargo install --force betterdisplay-kvm
launchctl kickstart -k "gui/$(id -u)/com.github.hacksore.betterdisplay-kvm"
```

`cargo install --force` replaces the installed binary in place. The `kickstart`
command restarts the LaunchAgent so the running daemon switches to the new
version immediately.

If you previously installed with `./install.sh` and want future upgrades to come
from Cargo, run `betterdisplay-kvm --install` once after `cargo install --force`.
That copies Cargo's installed binary into the stable per-user install location
and restarts the LaunchAgent.

## LaunchAgent status

Check whether the LaunchAgent process is running:

```bash
betterdisplay-kvm --status
```

The LaunchAgent label is `com.github.hacksore.betterdisplay-kvm`.

View recent daemon logs:

```bash
ls -t ~/Library/Logs/betterdisplay-kvm/betterdisplay-kvm*.log | head -1 | xargs tail -f
```

Restart the agent after changing config:

```bash
launchctl kickstart -k "gui/$(id -u)/com.github.hacksore.betterdisplay-kvm"
```

Stop the LaunchAgent when you want to run the daemon manually:

```bash
launchctl bootout "gui/$(id -u)/com.github.hacksore.betterdisplay-kvm"
betterdisplay-kvm --status
cargo run -- --launch
```

Start the LaunchAgent again when you are done:

```bash
launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/com.github.hacksore.betterdisplay-kvm.plist"
launchctl enable "gui/$(id -u)/com.github.hacksore.betterdisplay-kvm"
launchctl kickstart -k "gui/$(id -u)/com.github.hacksore.betterdisplay-kvm"
```

The generated plist lives at:

```text
~/Library/LaunchAgents/com.github.hacksore.betterdisplay-kvm.plist
```

## Uninstall

Run `./uninstall.sh`, and it will remove the program and clean everything up.
