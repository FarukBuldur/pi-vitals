use serde::Serialize;
use sysinfo::{Components, Disks, Networks, System};

#[derive(Serialize, Clone)]
pub struct Vitals {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub network: NetworkInfo,
    pub processes: Vec<ProcessInfo>,
    pub power: PowerInfo,
    pub uptime: u64,
    pub hostname: String,
}

#[derive(Serialize, Clone)]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub frequency_mhz: u64,
    pub core_count: usize,
    pub temperature_celsius: Option<f32>,
}

#[derive(Serialize, Clone)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub usage_percent: f32,
    pub swap_total_mb: u64,
    pub swap_used_mb: u64,
    pub swap_percent: f32,
}

#[derive(Serialize, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub usage_percent: f32,
}

#[derive(Serialize, Clone)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Serialize, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_kb_per_sec: f64,
    pub tx_kb_per_sec: f64,
}

#[derive(Serialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct PowerInfo {
    pub voltage: Option<f32>,
    pub throttled: bool,
    pub throttle_reason: String,
    pub current_watts: f32,
    pub hourly_wh: f32,
    pub monthly_kwh: f32,
}

pub struct VitalsCollector {
    sys: System,
    networks: Networks,
    last_rx: u64,
    last_tx: u64,
    last_tick: std::time::Instant,
}

impl VitalsCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let networks = Networks::new_with_refreshed_list();
        let (rx, tx) = total_network_bytes(&networks);
        Self {
            sys,
            networks,
            last_rx: rx,
            last_tx: tx,
            last_tick: std::time::Instant::now(),
        }
    }

    pub fn collect(&mut self) -> Vitals {
        self.sys.refresh_all();
        self.networks.refresh();

        let elapsed = self.last_tick.elapsed().as_secs_f64();
        self.last_tick = std::time::Instant::now();

        // CPU
        let cpu_usage = self.sys.global_cpu_info().cpu_usage();
        let freq = self.sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
        let core_count = self.sys.cpus().len();

        // Temperature via sysinfo Components
        let components = Components::new_with_refreshed_list();
        let temperature = components
            .iter()
            .find(|c| c.label().to_lowercase().contains("cpu") || c.label().to_lowercase().contains("core"))
            .map(|c| c.temperature());

        let cpu = CpuInfo {
            usage_percent: cpu_usage,
            frequency_mhz: freq,
            core_count,
            temperature_celsius: temperature,
        };

        // Memory
        let total_mem = self.sys.total_memory() / 1024 / 1024;
        let used_mem = self.sys.used_memory() / 1024 / 1024;
        let avail_mem = self.sys.available_memory() / 1024 / 1024;
        let mem_pct = (used_mem as f32 / total_mem as f32) * 100.0;
        let swap_total = self.sys.total_swap() / 1024 / 1024;
        let swap_used = self.sys.used_swap() / 1024 / 1024;
        let swap_pct = if swap_total > 0 {
            (swap_used as f32 / swap_total as f32) * 100.0
        } else {
            0.0
        };

        let memory = MemoryInfo {
            total_mb: total_mem,
            used_mb: used_mem,
            available_mb: avail_mem,
            usage_percent: mem_pct,
            swap_total_mb: swap_total,
            swap_used_mb: swap_used,
            swap_percent: swap_pct,
        };

        // Disks
        let disks_info = Disks::new_with_refreshed_list();
        let disks: Vec<DiskInfo> = disks_info
            .iter()
            .map(|d| {
                let total = d.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
                let avail = d.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
                let used = total - avail;
                let pct = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };
                DiskInfo {
                    name: d.name().to_string_lossy().to_string(),
                    mount: d.mount_point().to_string_lossy().to_string(),
                    total_gb: (total * 100.0).round() / 100.0,
                    used_gb: (used * 100.0).round() / 100.0,
                    usage_percent: pct,
                }
            })
            .collect();

        // Network
        let (cur_rx, cur_tx) = total_network_bytes(&self.networks);
        let rx_diff = cur_rx.saturating_sub(self.last_rx);
        let tx_diff = cur_tx.saturating_sub(self.last_tx);
        let rx_kbps = (rx_diff as f64 / 1024.0) / elapsed.max(0.1);
        let tx_kbps = (tx_diff as f64 / 1024.0) / elapsed.max(0.1);
        self.last_rx = cur_rx;
        self.last_tx = cur_tx;

        let interfaces: Vec<NetworkInterface> = self.networks
            .iter()
            .filter(|(name, _)| !name.starts_with("lo"))
            .map(|(name, data)| NetworkInterface {
                name: name.clone(),
                rx_bytes: data.total_received(),
                tx_bytes: data.total_transmitted(),
                rx_kb_per_sec: (rx_kbps * 100.0).round() / 100.0,
                tx_kb_per_sec: (tx_kbps * 100.0).round() / 100.0,
            })
            .collect();

        let network = NetworkInfo { interfaces };

        // Processes — top 10 by CPU
        let mut procs: Vec<ProcessInfo> = self.sys
            .processes()
            .values()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string(),
                cpu_percent: p.cpu_usage(),
                memory_mb: p.memory() / 1024 / 1024,
                status: format!("{:?}", p.status()),
            })
            .collect();
        procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
        procs.truncate(10);

        // Power (Pi-specific via vcgencmd)
        let power = read_power_info(cpu_usage);

        // Uptime & hostname
        let uptime = System::uptime();
        let hostname = System::host_name().unwrap_or_else(|| "raspberrypi".to_string());

        Vitals { cpu, memory, disks, network, processes: procs, power, uptime, hostname }
    }
}

fn total_network_bytes(networks: &Networks) -> (u64, u64) {
    networks.iter().fold((0, 0), |(rx, tx), (_, d)| {
        (rx + d.total_received(), tx + d.total_transmitted())
    })
}

fn read_power_info(cpu_usage: f32) -> PowerInfo {
    let throttled_raw = std::process::Command::new("vcgencmd")
        .arg("get_throttled")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let throttle_val = throttled_raw
        .trim()
        .strip_prefix("throttled=0x")
        .and_then(|s| u32::from_str_radix(s, 16).ok())
        .unwrap_or(0);

    let throttled = throttle_val != 0;
    let throttle_reason = match throttle_val {
        0 => "None".to_string(),
        v if v & 0x1 != 0 => "Under-voltage detected".to_string(),
        v if v & 0x2 != 0 => "Frequency capped".to_string(),
        v if v & 0x4 != 0 => "Currently throttled".to_string(),
        v if v & 0x8 != 0 => "Soft temperature limit".to_string(),
        _ => format!("Unknown (0x{:x})", throttle_val),
    };

    let voltage = std::process::Command::new("vcgencmd")
        .arg("measure_volts")
        .arg("core")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.trim()
                .strip_prefix("volt=")
                .and_then(|v| v.strip_suffix("V"))
                .and_then(|v| v.parse::<f32>().ok())
        });

    // Estimate power draw from CPU load.
    // Typical Pi 4 at 5V USB: ~2.5W idle, ~6.4W full load.
    // Pi 5: ~2.7W idle, ~12W full load.
    // Try reading actual current from the PMIC (Pi 5) first.
    let current_watts = read_pmic_watts()
        .unwrap_or_else(|| estimate_watts_from_cpu(cpu_usage));

    let hourly_wh = current_watts;
    let monthly_kwh = current_watts * 24.0 * 30.0 / 1000.0;

    PowerInfo { voltage, throttled, throttle_reason, current_watts, hourly_wh, monthly_kwh }
}

/// Try to read actual power from Pi 5 PMIC via `vcgencmd pmic_read_adc`.
/// Returns total board power in watts if available.
fn read_pmic_watts() -> Option<f32> {
    let output = std::process::Command::new("vcgencmd")
        .args(["pmic_read_adc"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())?;

    // Output lines look like: "VDD_CORE_A current(0.3280)A" / "VDD_CORE_V volt(0.7250)V"
    // Sum up all power (V*A) pairs for each rail, or just sum current lines × 5V input.
    let mut total_amps: f32 = 0.0;
    for line in output.lines() {
        if line.contains("current(") {
            if let Some(val) = line
                .split("current(")
                .nth(1)
                .and_then(|s| s.strip_suffix(")A"))
                .and_then(|s| s.parse::<f32>().ok())
            {
                total_amps += val;
            }
        }
    }

    if total_amps > 0.0 {
        Some(5.0 * total_amps)
    } else {
        None
    }
}

/// Fallback estimation: interpolate between idle and max watts based on CPU usage.
/// Based on measured Raspberry Pi 4 Model B power draw.
const PI_IDLE_WATTS: f32 = 2.5;
const PI_MAX_WATTS: f32 = 6.4;

fn estimate_watts_from_cpu(cpu_usage_pct: f32) -> f32 {
    let load = (cpu_usage_pct / 100.0).clamp(0.0, 1.0);
    PI_IDLE_WATTS + load * (PI_MAX_WATTS - PI_IDLE_WATTS)
}