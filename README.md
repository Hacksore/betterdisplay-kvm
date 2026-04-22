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

Run `./install.sh`, and it will install a LaunchAgent and start the program.

## Uninstall

Run `./uninstall.sh`, and it will remove the program and clean everything up.

