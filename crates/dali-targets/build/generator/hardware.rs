use super::super::manifest::{
    Capabilities, Clock, Dfu, DriverProbe, I2c, IsolationMemory, Memory, Pin, Scheduler, Storage,
    TargetMemoryRegion, UserKey, Watchdog,
};
use super::super::render::string_literal;

pub(super) fn generate_scheduler(scheduler: &Scheduler) -> String {
    format!(
        "SchedulerProfile {{ quantum_ticks: {}, tick_hz: {} }}",
        scheduler.quantum_ticks, scheduler.tick_hz
    )
}

pub(super) fn generate_i2c(i2c: &I2c) -> String {
    format!(
        "I2cProfile {{ bus_frequency_hz: {} }}",
        i2c.bus_frequency_hz
    )
}

pub(super) fn generate_display(display: &super::super::manifest::Display) -> String {
    format!(
        "DisplayProfile {{ controller: {}, i2c_address: {}, width: {}, height: {}, text_cell_width: {}, text_cell_height: {}, timeout_ticks: {} }}",
        string_literal(&display.controller),
        display.i2c_address,
        display.width,
        display.height,
        display.text_cell_width,
        display.text_cell_height,
        display.timeout_ticks,
    )
}

pub(super) fn generate_driver_probe(probe: &DriverProbe) -> String {
    format!(
        "DriverProbeProfile {{ uart_baud_rate_hz: {}, spi_clock_hz: {}, timeout_ticks: {}, polls_per_timeout_tick: {}, timer_timeout_divisor: {}, timer_evidence_poll_limit: {}, buffer_length: {}, spi_fill_byte: {}, spi_device_id: {}, i2c_address: {} }}",
        probe.uart_baud_rate_hz,
        probe.spi_clock_hz,
        probe.timeout_ticks,
        probe.polls_per_timeout_tick,
        probe.timer_timeout_divisor,
        probe.timer_evidence_poll_limit,
        probe.buffer_length,
        probe.spi_fill_byte,
        probe.spi_device_id,
        probe.i2c_address,
    )
}

pub(super) fn generate_user_key(user_key: &UserKey) -> String {
    let port = user_key
        .port
        .chars()
        .next()
        .expect("user-key port validated");
    format!("UserKeyProfile {{ port: {port:?}, pin: {} }}", user_key.pin)
}

pub(super) fn generate_watchdog(watchdog: &Watchdog) -> String {
    format!(
        "WatchdogProfile {{ controller: {}, timeout_ms: {}, feed_interval_ms: {}, reset_cause_supported: {} }}",
        string_literal(&watchdog.controller),
        watchdog.timeout_ms,
        watchdog.feed_interval_ms,
        watchdog.reset_cause_supported
    )
}

pub(super) fn generate_capabilities(capabilities: &Capabilities) -> String {
    format!(
        "CapabilitiesProfile {{ storage: {}, usb_console: {}, mpu: {}, relocation: {} }}",
        capabilities.storage, capabilities.usb_console, capabilities.mpu, capabilities.relocation
    )
}

pub(super) fn generate_dfu(dfu: &Dfu) -> String {
    format!(
        "DfuProfile {{ vendor_id: 0x{:04X}, product_id: 0x{:04X}, address: 0x{:08X}, alternate: {}, leave: {} }}",
        dfu.vendor_id, dfu.product_id, dfu.address, dfu.alternate, dfu.leave
    )
}

pub(super) fn generate_clock(clock: &Clock) -> String {
    format!(
        "ClockProfile {{ source: {}, input_hz: {}, system_hz: {}, pclk1_hz: {}, pclk2_hz: {}, usb_hz: {} }}",
        string_literal(&clock.source),
        clock.input_hz,
        clock.system_hz,
        clock.pclk1_hz,
        clock.pclk2_hz,
        clock.usb_hz
    )
}

pub(super) fn generate_memory(memory: &Memory) -> String {
    format!(
        "MemoryProfile {{ flash: {}, kernel_origin: 0x{:08X}, kernel_length: {}, application_origin: 0x{:08X}, application_length: {}, runtime_origin: 0x{:08X}, runtime_length: {}, dma: {}, ccm: {}, isolation: {} }}",
        generate_target_memory_region(&memory.flash),
        memory.kernel_origin,
        memory.kernel_length,
        memory.application_origin,
        memory.application_length,
        memory.runtime_origin,
        memory.runtime_length,
        generate_target_memory_region(&memory.dma),
        memory
            .ccm
            .as_ref()
            .map(generate_target_memory_region)
            .map_or_else(|| "None".to_owned(), |value| format!("Some({value})")),
        memory
            .isolation
            .as_ref()
            .map(generate_isolation_memory)
            .map_or_else(|| "None".to_owned(), |value| format!("Some({value})")),
    )
}

fn generate_target_memory_region(region: &TargetMemoryRegion) -> String {
    format!(
        "TargetMemoryRegion {{ origin: 0x{:08X}, length: {} }}",
        region.origin, region.length
    )
}

fn generate_isolation_memory(memory: &IsolationMemory) -> String {
    format!(
        "IsolationMemoryProfile {{ peripheral_origin: {}, peripheral_length: {}, bus_fault_origin: {}, bus_fault_length: {}, slots: &[{}] }}",
        memory
            .peripheral_origin
            .map_or_else(|| "None".to_owned(), |value| format!("Some(0x{value:08X})")),
        memory
            .peripheral_length
            .map_or_else(|| "None".to_owned(), |value| format!("Some({value})")),
        memory
            .bus_fault_origin
            .map_or_else(|| "None".to_owned(), |value| format!("Some(0x{value:08X})")),
        memory
            .bus_fault_length
            .map_or_else(|| "None".to_owned(), |value| format!("Some({value})")),
        memory
            .slots
            .iter()
            .map(generate_isolation_slot)
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn generate_isolation_slot(slot: &super::super::manifest::IsolationSlot) -> String {
    format!(
        "IsolationSlot {{ id: {}, name: {}, code_origin: 0x{:08X}, code_length: {}, data_origin: 0x{:08X}, data_length: {}, stack_length: {} }}",
        slot.id,
        string_literal(&slot.name),
        slot.code_origin,
        slot.code_length,
        slot.data_origin,
        slot.data_length,
        slot.stack_length,
    )
}

pub(super) fn generate_pin(pin: &Pin) -> String {
    format!(
        "PinProfile {{ port: {}, number: {}, alternate_function: {}, active_high: {} }}",
        string_literal(&pin.port),
        pin.number,
        pin.alternate_function,
        pin.active_high
    )
}

pub(super) fn generate_usb(usb: &super::super::manifest::Usb) -> String {
    format!(
        "UsbProfile {{ controller: {}, dm: {}, dp: {} }}",
        string_literal(&usb.controller),
        generate_pin(&usb.dm),
        generate_pin(&usb.dp)
    )
}

pub(super) fn generate_storage(storage: &Storage) -> String {
    let data = storage
        .data
        .iter()
        .map(generate_pin)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "StorageProfile {{ controller: {}, bus_width: {}, data_timeout_cycles: {}, command_poll_limit: {}, data_poll_limit: {}, dma_stop_poll_limit: {}, clock: {}, command: {}, data: [{}] }}",
        string_literal(&storage.controller),
        storage.bus_width,
        storage.data_timeout_cycles,
        storage.command_poll_limit,
        storage.data_poll_limit,
        storage.dma_stop_poll_limit,
        generate_pin(&storage.clock),
        generate_pin(&storage.command),
        data
    )
}
