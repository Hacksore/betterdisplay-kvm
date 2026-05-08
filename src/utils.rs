use comfy_table::{
  Attribute, Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
  presets::UTF8_FULL,
};
use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode};
use log::{debug, error, info};
use lunchctl::LaunchAgent;
use serde::{Deserialize, Serialize};
use std::{
  fs,
  io::{self, IsTerminal},
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{self, Command, Output},
};

pub const DEFAULT_DEVICE_ID: &str = "046d:c547";
pub const LAUNCH_AGENT_LABEL: &str = "com.github.hacksore.betterdisplay-kvm";
const BIN_NAME: &str = "betterdisplay-kvm";

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
  /// The USB device id in the form "vvvv:pppp"
  pub usb_device_id: Option<String>,
  /// DDC input code for system 1 (e.g. 15)
  pub system_one_input: Option<u16>,
  /// DDC input code for system 2 (e.g. 18)
  pub system_two_input: Option<u16>,
  /// Log level: "error", "warn", "info", "debug", "trace"
  pub log_level: Option<String>,
  /// Enable alternative DDC flag for LG monitors (adds --ddcAlt)
  pub ddc_alt: Option<bool>,
}

impl AppConfig {
  pub fn with_defaults(self) -> ResolvedConfig {
    ResolvedConfig {
      usb_device_id: self
        .usb_device_id
        .unwrap_or_else(|| DEFAULT_DEVICE_ID.to_string()),
      system_one_input: self.system_one_input.unwrap_or(15),
      system_two_input: self.system_two_input.unwrap_or(18),
      log_level: self.log_level.unwrap_or_else(|| "info".to_string()),
      ddc_alt: self.ddc_alt.unwrap_or(false),
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedConfig {
  pub usb_device_id: String,
  pub system_one_input: u16,
  pub system_two_input: u16,
  pub log_level: String,
  pub ddc_alt: bool,
}

pub fn get_betterdisplay_path() -> PathBuf {
  if let Ok(override_path) = std::env::var("BETTERDISPLAYCLI_PATH") {
    let p = PathBuf::from(override_path);
    if p.exists() {
      return p;
    }
  }

  let common_candidates = [
    "/opt/homebrew/bin/betterdisplaycli",
    "/usr/local/bin/betterdisplaycli",
    "/usr/bin/betterdisplaycli",
    "/bin/betterdisplaycli",
  ];
  for candidate in common_candidates {
    let p = Path::new(candidate);
    if p.exists() {
      return p.to_path_buf();
    }
  }

  if let Some(path_var) = std::env::var_os("PATH") {
    for dir in std::env::split_paths(&path_var) {
      let p = dir.join("betterdisplaycli");
      if p.exists() {
        return p;
      }
    }
  }

  error!(
    "Could not locate 'betterdisplaycli'. Set BETTERDISPLAYCLI_PATH or install to /opt/homebrew/bin or /usr/local/bin."
  );
  process::exit(1);
}

pub fn set_input(input_code: u16, use_ddc_alt: bool) -> anyhow::Result<()> {
  let betterdisplay_path = get_betterdisplay_path();

  // TODO: figure out how to make this path dynamic or configurable
  let mut cmd = Command::new(betterdisplay_path);
  cmd.arg("set");
  if use_ddc_alt {
    cmd.arg("--ddcAlt");
  }
  cmd.args([
    format!("--ddc={}", input_code),
    "--vcp=inputSelect".to_string(),
  ]);

  debug!("Executing betterdisplaycli command: {:?}", cmd);

  let mut child = cmd
    .spawn()
    .map_err(|e| anyhow::anyhow!("Failed to execute betterdisplaycli process: {}", e))?;

  let status = child
    .wait()
    .map_err(|e| anyhow::anyhow!("Failed to wait for betterdisplaycli process: {}", e))?;

  if !status.success() {
    return Err(anyhow::anyhow!(
      "betterdisplaycli exited with status: {}",
      status
    ));
  }

  debug!("Successfully executed betterdisplaycli command");
  Ok(())
}

pub fn on_connect(cfg: &ResolvedConfig) {
  info!("switch input to the system_one_input");
  if let Err(e) = set_input(cfg.system_one_input, cfg.ddc_alt) {
    error!("Failed to set input on connect: {}", e);
  }
}

pub fn on_disconnect(cfg: &ResolvedConfig) {
  info!("switch input to system_two_input");
  if let Err(e) = set_input(cfg.system_two_input, cfg.ddc_alt) {
    error!("Failed to set input on disconnect: {}", e);
  }
}

pub fn load_config() -> anyhow::Result<ResolvedConfig> {
  let oshome = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
  let mut config_path = PathBuf::from(oshome);
  config_path.push(".config");
  config_path.push("betterdisplay-kvm");
  config_path.push("config.toml");

  let builder =
    config::Config::builder().add_source(config::File::from(config_path.clone()).required(false));

  let cfg: AppConfig = builder.build()?.try_deserialize()?;

  if !config_path.exists() {
    if let Some(parent) = config_path.parent() {
      if !parent.exists() {
        fs::create_dir_all(parent)?;
      }
    }
    let resolved = cfg.with_defaults();
    fs::write(&config_path, toml::to_string_pretty(&resolved)?)?;
    return Ok(resolved);
  }

  Ok(cfg.with_defaults())
}

pub fn handle_launch_agent() -> anyhow::Result<()> {
  info!("Installing launch agent since --install was passed...");
  let mut agent = LaunchAgent::new(LAUNCH_AGENT_LABEL);
  let executable_path = install_current_executable()?;

  agent.program_arguments = vec![
    executable_path.to_string_lossy().to_string(),
    String::from("--launch"),
  ];
  agent.run_at_load = true;
  agent.keep_alive = true;

  agent.write()?;
  refresh_launch_agent(&agent)?;

  info!("Launch agent installed and started.");

  process::exit(0);
}

fn install_current_executable() -> anyhow::Result<PathBuf> {
  let current_exe = std::env::current_exe()
    .map_err(|e| anyhow::anyhow!("Failed to resolve current executable path: {}", e))?;
  let install_path = installed_executable_path()?;

  if paths_point_to_same_file(&current_exe, &install_path) {
    return Ok(install_path);
  }

  if let Some(parent) = install_path.parent() {
    fs::create_dir_all(parent)?;
  }

  fs::copy(&current_exe, &install_path).map_err(|e| {
    anyhow::anyhow!(
      "Failed to install executable from {} to {}: {}",
      current_exe.display(),
      install_path.display(),
      e
    )
  })?;
  fs::set_permissions(&install_path, fs::Permissions::from_mode(0o755))?;

  Ok(install_path)
}

fn installed_executable_path() -> anyhow::Result<PathBuf> {
  let mut path = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
  path.push("Library");
  path.push("Application Support");
  path.push(BIN_NAME);
  path.push(BIN_NAME);
  Ok(path)
}

fn paths_point_to_same_file(left: &Path, right: &Path) -> bool {
  match (left.canonicalize(), right.canonicalize()) {
    (Ok(left), Ok(right)) => left == right,
    _ => false,
  }
}

fn refresh_launch_agent(agent: &LaunchAgent) -> anyhow::Result<()> {
  let user_id = get_current_user_id()?;
  let target = format!("gui/{}/{}", user_id, LAUNCH_AGENT_LABEL);
  let domain = format!("gui/{}", user_id);
  let plist_path = agent.path();

  let _ = Command::new("launchctl")
    .args(["bootout", &target])
    .output();

  run_launchctl(["bootstrap", &domain, &plist_path.to_string_lossy()])?;
  run_launchctl(["enable", &target])?;
  run_launchctl(["kickstart", "-k", &target])?;

  Ok(())
}

fn run_launchctl<const N: usize>(args: [&str; N]) -> anyhow::Result<()> {
  let output = Command::new("launchctl").args(args).output()?;
  if !output.status.success() {
    return Err(anyhow::anyhow!(
      "launchctl failed with status {}: {}{}",
      output.status,
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr)
    ));
  }

  Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchAgentStatus {
  Running { pid: Option<String> },
  LoadedNotRunning,
  NotLoaded,
}

pub fn get_launch_agent_status() -> anyhow::Result<LaunchAgentStatus> {
  let user_id = get_current_user_id()?;
  let target = format!("gui/{}/{}", user_id, LAUNCH_AGENT_LABEL);
  let output = Command::new("launchctl")
    .args(["print", &target])
    .output()?;

  Ok(parse_launch_agent_status(&output))
}

pub fn print_launch_agent_status() -> anyhow::Result<()> {
  let status = get_launch_agent_status()?;
  println!(
    "{}",
    format_launch_agent_status(&status, io::stdout().is_terminal())
  );

  Ok(())
}

fn format_launch_agent_status(status: &LaunchAgentStatus, use_color: bool) -> String {
  let (state, indicator, pid, detail, status_color) = match status {
    LaunchAgentStatus::Running { pid } => (
      "Running",
      "OK",
      pid.as_deref().unwrap_or("-"),
      "LaunchAgent is loaded and the daemon is running.",
      Color::Green,
    ),
    LaunchAgentStatus::LoadedNotRunning => (
      "Loaded",
      "WARN",
      "-",
      "LaunchAgent is loaded, but the daemon is not running.",
      Color::Yellow,
    ),
    LaunchAgentStatus::NotLoaded => (
      "Not loaded",
      "OFF",
      "-",
      "LaunchAgent is not loaded or not running.",
      Color::Red,
    ),
  };

  let mut table = Table::new();
  if use_color {
    table.enforce_styling();
  }

  table
    .load_preset(UTF8_FULL)
    .apply_modifier(UTF8_ROUND_CORNERS)
    .set_content_arrangement(ContentArrangement::Dynamic)
    .set_header(vec![
      Cell::new("betterdisplay-kvm status").add_attribute(Attribute::Bold),
      Cell::new("Value").add_attribute(Attribute::Bold),
    ])
    .add_row(vec![Cell::new("Service"), Cell::new(LAUNCH_AGENT_LABEL)])
    .add_row(vec![
      Cell::new("Indicator"),
      status_cell(indicator, status_color, use_color),
    ])
    .add_row(vec![
      Cell::new("Status"),
      status_cell(state, status_color, use_color),
    ])
    .add_row(vec![Cell::new("PID"), Cell::new(pid)])
    .add_row(vec![Cell::new("Detail"), Cell::new(detail)]);

  table.to_string()
}

fn status_cell(value: &str, color: Color, use_color: bool) -> Cell {
  let cell = Cell::new(value).add_attribute(Attribute::Bold);
  if use_color { cell.fg(color) } else { cell }
}

fn get_current_user_id() -> anyhow::Result<String> {
  let output = Command::new("id").arg("-u").output()?;
  if !output.status.success() {
    return Err(anyhow::anyhow!(
      "Failed to determine current user id: {}",
      String::from_utf8_lossy(&output.stderr).trim()
    ));
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_launch_agent_status(output: &Output) -> LaunchAgentStatus {
  if !output.status.success() {
    return LaunchAgentStatus::NotLoaded;
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  if stdout.contains("state = running") || stdout.contains("job state = running") {
    return LaunchAgentStatus::Running {
      pid: extract_launchctl_value(&stdout, "pid = "),
    };
  }

  LaunchAgentStatus::LoadedNotRunning
}

fn extract_launchctl_value(output: &str, prefix: &str) -> Option<String> {
  output
    .lines()
    .map(str::trim)
    .find_map(|line| line.strip_prefix(prefix))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToString::to_string)
}

pub fn setup_logger(cfg: &ResolvedConfig) -> anyhow::Result<()> {
  let mut logs_dir =
    dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
  logs_dir.push("Library");
  logs_dir.push("Logs");
  logs_dir.push("betterdisplay-kvm");

  if !logs_dir.exists() {
    fs::create_dir_all(&logs_dir)?;
  }

  let level_str = match cfg.log_level.to_lowercase().as_str() {
    "error" => "error",
    "warn" | "warning" => "warn",
    "info" => "info",
    "debug" => "debug",
    "trace" => "trace",
    _ => "info",
  };

  let spec = format!("off,betterdisplay_kvm={}", level_str);

  Logger::try_with_str(spec)?
    .log_to_file(
      FileSpec::default()
        .directory(&logs_dir)
        .basename("betterdisplay-kvm")
        .suffix("log"),
    )
    .format_for_files(flexi_logger::detailed_format)
    .duplicate_to_stdout(Duplicate::All)
    .duplicate_to_stderr(Duplicate::Error)
    .format_for_stdout(flexi_logger::detailed_format)
    .write_mode(WriteMode::Direct)
    .rotate(
      Criterion::Size(10_000_000),
      Naming::Timestamps,
      Cleanup::KeepLogFiles(7),
    )
    .start()?;

  Ok(())
}
