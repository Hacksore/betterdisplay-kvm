use crate::utils::{ResolvedConfig, on_connect, on_disconnect};
use futures_lite::stream::StreamExt;
use log::{debug, error, info};
use nusb::hotplug::HotplugEvent;
use nusb::{DeviceId, DeviceInfo, MaybeFuture};
use std::collections::HashMap;

/// All connected USB devices, sorted by vendor id, product id, then serial.
pub fn list_usb_devices() -> anyhow::Result<Vec<DeviceInfo>> {
  let mut device_list: Vec<DeviceInfo> = nusb::list_devices().wait()?.collect();
  device_list.sort_by(|a, b| {
    (
      a.vendor_id(),
      a.product_id(),
      a.serial_number().unwrap_or(""),
    )
      .cmp(&(
        b.vendor_id(),
        b.product_id(),
        b.serial_number().unwrap_or(""),
      ))
  });
  Ok(device_list)
}

/// One line per device: `usb_device_id`, manufacturer, product, serial (tab-separated).
pub fn format_usb_device_line(info: &DeviceInfo) -> String {
  format!(
    "{:04x}:{:04x}\t{}\t{}\t{}",
    info.vendor_id(),
    info.product_id(),
    info.manufacturer_string().unwrap_or(""),
    info.product_string().unwrap_or(""),
    info.serial_number().unwrap_or(""),
  )
}

pub struct DeviceManager {
  devices: HashMap<DeviceId, (u16, u16)>,
  config: ResolvedConfig,
}

impl DeviceManager {
  pub fn new(config: ResolvedConfig) -> Self {
    Self {
      devices: HashMap::new(),
      config,
    }
  }

  /// Enumerate all currently connected USB devices
  pub fn enumerate_devices(&mut self) -> anyhow::Result<()> {
    debug!("Enumerating all USB devices");

    for info in list_usb_devices()? {
      let id = info.id();
      let vendor = info.vendor_id();
      let product = info.product_id();
      let manufacturer_name = info.manufacturer_string().unwrap_or("Unknown Manufacturer");
      let device_name = info.product_string().unwrap_or("Unknown Product");
      let device_str = format!("{:04x}:{:04x}", vendor, product);

      self.devices.insert(id, (vendor, product));

      debug!(
        "{:?} ({:?}): {}",
        device_name, manufacturer_name, device_str
      );

      if device_str == self.config.usb_device_id {
        info!(
          "Configured USB device {} found, switching input to {}",
          device_str, self.config.system_one_input
        );
        on_connect(&self.config);
      }
    }

    Ok(())
  }

  /// Start monitoring USB device events
  pub async fn monitor_devices(&mut self) -> anyhow::Result<()> {
    let mut events = nusb::watch_devices().map_err(|e| {
      error!("Failed to start USB device monitoring: {}", e);
      anyhow::anyhow!("Failed to start USB device monitoring: {}", e)
    })?;

    while let Some(event) = events.next().await {
      self.handle_event(event)?;
    }

    Ok(())
  }

  fn handle_event(&mut self, event: HotplugEvent) -> anyhow::Result<()> {
    match event {
      HotplugEvent::Connected(info) => {
        self.handle_device_connected(info);
      }
      HotplugEvent::Disconnected(id) => {
        self.handle_device_disconnected(id);
      }
    }
    Ok(())
  }

  fn handle_device_connected(&mut self, info: DeviceInfo) {
    let id = info.id();
    let vendor = info.vendor_id();
    let product = info.product_id();
    let device_str = format!("{:04x}:{:04x}", vendor, product);

    debug!("Connected USB device: {}", device_str);

    if device_str == self.config.usb_device_id {
      on_connect(&self.config);
    }

    self.devices.insert(id, (vendor, product));
    debug!("Added device to cache: {}", device_str);
  }

  fn handle_device_disconnected(&mut self, id: DeviceId) {
    if let Some((vendor, product)) = self.devices.remove(&id) {
      let device_str = format!("{:04x}:{:04x}", vendor, product);
      debug!("Disconnected USB device: {}", device_str);

      if device_str == self.config.usb_device_id {
        debug!("Configured device disconnected, switching to system_two_input");
        on_disconnect(&self.config);
      }

      debug!("Removed device from cache: {}", device_str);
    } else {
      error!("Unknown device disconnected: {:?}", id);
    }
  }
}
