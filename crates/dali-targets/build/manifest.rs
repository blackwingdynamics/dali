use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Manifest {
    pub(super) profile: Profile,
    pub(super) artifacts: Option<Artifacts>,
    pub(super) clock: Clock,
    pub(super) i2c: I2c,
    pub(super) display: Display,
    pub(super) driver_probe: DriverProbe,
    pub(super) memory: Memory,
    pub(super) status_led: Pin,
    pub(super) user_key: UserKey,
    pub(super) usb: Usb,
    pub(super) storage: Option<Storage>,
    pub(super) scheduler: Option<Scheduler>,
    pub(super) watchdog: Option<Watchdog>,
    pub(super) authentication: Authentication,
    pub(super) capabilities: Capabilities,
}

#[derive(Debug, Deserialize)]
pub(super) struct Artifacts {
    pub(super) kernel_binary: String,
    pub(super) kernel_elf: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Profile {
    pub(super) name: String,
    pub(super) backend: String,
    pub(super) board: String,
    pub(super) mcu: String,
    pub(super) rust_target: String,
    pub(super) probe_chip: Option<String>,
    pub(super) dfu: Option<Dfu>,
    pub(super) application_supported: bool,
    pub(super) amrn_target_id: u8,
    pub(super) abi_version: u8,
}

#[derive(Debug, Deserialize)]
pub(super) struct Dfu {
    pub(super) vendor_id: u16,
    pub(super) product_id: u16,
    pub(super) address: u32,
    pub(super) alternate: u8,
    pub(super) leave: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct Clock {
    pub(super) source: String,
    pub(super) input_hz: u32,
    pub(super) system_hz: u32,
    pub(super) pclk1_hz: u32,
    pub(super) pclk2_hz: u32,
    pub(super) usb_hz: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct I2c {
    pub(super) bus_frequency_hz: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct Display {
    pub(super) controller: String,
    pub(super) i2c_address: u8,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) text_cell_width: usize,
    pub(super) text_cell_height: usize,
    pub(super) timeout_ticks: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct DriverProbe {
    pub(super) uart_baud_rate_hz: u32,
    pub(super) spi_clock_hz: u32,
    pub(super) timeout_ticks: u32,
    pub(super) polls_per_timeout_tick: u32,
    pub(super) timer_timeout_divisor: u32,
    pub(super) timer_evidence_poll_limit: u32,
    pub(super) buffer_length: usize,
    pub(super) spi_fill_byte: u8,
    pub(super) spi_device_id: u8,
    pub(super) i2c_address: u8,
}

#[derive(Debug, Deserialize)]
pub(super) struct UserKey {
    pub(super) port: String,
    pub(super) pin: u8,
}

#[derive(Debug, Deserialize)]
pub(super) struct Scheduler {
    pub(super) quantum_ticks: u32,
    pub(super) tick_hz: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct Watchdog {
    pub(super) controller: String,
    pub(super) timeout_ms: u32,
    pub(super) feed_interval_ms: u32,
    pub(super) reset_cause_supported: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct Authentication {
    pub(super) development: String,
    pub(super) release: String,
    #[serde(default)]
    pub(super) development_trust_anchors: Vec<TrustAnchor>,
    #[serde(default)]
    pub(super) release_trust_anchors: Vec<TrustAnchor>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TrustAnchor {
    pub(super) key_id: String,
    pub(super) public_key: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Memory {
    pub(super) flash: TargetMemoryRegion,
    pub(super) firmware: TargetMemoryRegion,
    pub(super) artifact: Option<TargetMemoryRegion>,
    pub(super) artifact_capacity: Option<u32>,
    pub(super) kernel_origin: u32,
    pub(super) kernel_length: u32,
    pub(super) application_origin: u32,
    pub(super) application_length: u32,
    pub(super) runtime_origin: u32,
    pub(super) runtime_length: u32,
    pub(super) dma: TargetMemoryRegion,
    pub(super) ccm: Option<TargetMemoryRegion>,
    pub(super) isolation: Option<IsolationMemory>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TargetMemoryRegion {
    pub(super) origin: u32,
    pub(super) length: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct IsolationMemory {
    pub(super) peripheral_origin: Option<u32>,
    pub(super) peripheral_length: Option<u32>,
    pub(super) bus_fault_origin: Option<u32>,
    pub(super) bus_fault_length: Option<u32>,
    #[serde(default)]
    pub(super) slots: Vec<IsolationSlot>,
}

#[derive(Debug, Deserialize)]
pub(super) struct IsolationSlot {
    pub(super) id: u8,
    pub(super) name: String,
    pub(super) code_origin: u32,
    pub(super) code_length: u32,
    pub(super) data_origin: u32,
    pub(super) data_length: u32,
    pub(super) stack_length: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct Pin {
    pub(super) port: String,
    pub(super) number: u8,
    pub(super) alternate_function: u8,
    pub(super) active_high: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct Usb {
    pub(super) controller: String,
    pub(super) dm: Pin,
    pub(super) dp: Pin,
}

#[derive(Debug, Deserialize)]
pub(super) struct Storage {
    pub(super) controller: String,
    pub(super) bus_width: u8,
    pub(super) data_timeout_cycles: u32,
    pub(super) command_poll_limit: u32,
    pub(super) ocr_poll_limit: u32,
    pub(super) data_poll_limit: u32,
    pub(super) dma_stop_poll_limit: u32,
    pub(super) clock: Pin,
    pub(super) command: Pin,
    pub(super) data: [Pin; 4],
}

#[derive(Debug, Deserialize)]
pub(super) struct Capabilities {
    pub(super) storage: bool,
    pub(super) usb_console: bool,
    pub(super) mpu: bool,
    pub(super) relocation: bool,
}
