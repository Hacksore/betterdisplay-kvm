use crate::utils::{ResolvedConfig, load_config, prime_betterdisplay_path_cache, setup_logger};
use log::{debug, error, info};
use std::panic;

pub struct App {
  config: ResolvedConfig,
}

impl App {
  /// Initialize the application with configuration and logging
  pub fn initialize() -> anyhow::Result<Self> {
    // Load config first to get the log level
    let config = load_config().map_err(|e| {
      eprintln!("Failed to load config: {}", e);
      e
    })?;

    // Set up logger with the proper log level from config
    setup_logger(&config)?;

    info!("betterdisplay-kvm starting...");
    debug!("Configuration loaded: {:?}", config);

    // Set up panic hook to capture panics and log them
    debug!("Installing panic hook");
    Self::setup_panic_hook();

    debug!("Preflighting betterdisplaycli path");
    prime_betterdisplay_path_cache();
    debug!("Application initialization complete");

    Ok(Self { config })
  }

  /// Get the resolved configuration
  pub fn config(&self) -> &ResolvedConfig {
    &self.config
  }

  fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
      error!("PANIC: {}", panic_info);
      if let Some(location) = panic_info.location() {
        error!(
          "Location: {}:{}:{}",
          location.file(),
          location.line(),
          location.column()
        );
      }
      if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        error!("Message: {}", s);
      } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
        error!("Message: {}", s);
      }
    }));
  }
}
