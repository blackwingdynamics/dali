#![no_std]

//! Typed target metadata generated from repository board manifests.

/// Metadata required to build and validate an application for a Dali target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetProfile {
    /// Stable profile name used by Dali commands.
    pub name: &'static str,
    /// Backend identifier used to select the hardware implementation.
    pub backend: &'static str,
    /// Generated Rust registry constant used by board scaffolds.
    pub registry_constant: &'static str,
    /// Human-readable board name.
    pub board: &'static str,
    /// MCU identifier supplied by the board manufacturer.
    pub mcu: &'static str,
    /// Rust compilation target triple.
    pub rust_target: &'static str,
    /// Conventional kernel firmware artifact name, when declared.
    pub kernel_binary: Option<&'static str>,
    /// Conventional kernel ELF artifact name, when declared.
    pub kernel_elf: Option<&'static str>,
    /// Probe chip identifier used by the debug transport, when declared.
    pub probe_chip: Option<&'static str>,
    /// USB DFU identity used by the firmware download transport, when declared.
    pub dfu: Option<DfuProfile>,
    /// Whether the profile is currently valid for AMRN application execution.
    pub application_supported: bool,
    /// AMRN target identifier assigned by the package contract.
    pub amrn_target_id: u8,
    /// Application ABI version.
    pub abi_version: u8,
    /// Platform capabilities declared by the target manifest.
    pub capabilities: CapabilitiesProfile,
    /// Board clock metadata.
    pub clock: ClockProfile,
    /// Board memory regions used by the kernel and applications.
    pub memory: MemoryProfile,
    /// Logical status LED metadata.
    pub status_led: PinProfile,
    /// USB FS data-pin metadata.
    pub usb: UsbProfile,
    /// Storage bus metadata.
    pub storage: Option<StorageProfile>,
    /// Scheduler configuration declared by the target manifest.
    pub scheduler: Option<SchedulerProfile>,
}

/// Optional hardware and runtime capabilities declared by a target profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilitiesProfile {
    /// Whether the selected backend provides the declared storage path.
    pub storage: bool,
    /// Whether the target provides the Dali USB console path.
    pub usb_console: bool,
    /// Whether the processor/backend can enforce the declared MPU boundary.
    pub mpu: bool,
    /// Whether the selected ABI supports relocation packages.
    pub relocation: bool,
}

/// USB DFU identity declared by a target manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DfuProfile {
    /// USB vendor identifier.
    pub vendor_id: u16,
    /// USB product identifier.
    pub product_id: u16,
    /// Flash download address accepted by the DFU target.
    pub address: u32,
    /// DFU alternate interface used for the firmware image.
    pub alternate: u8,
    /// Whether the DFU tool should request runtime transition after download.
    pub leave: bool,
}

/// Clock values declared by a board manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockProfile {
    /// Clock source declared by the board manifest.
    pub source: &'static str,
    /// Input clock frequency in hertz.
    pub input_hz: u32,
    /// Target system clock frequency in hertz.
    pub system_hz: u32,
    /// APB1 peripheral clock frequency in hertz.
    pub pclk1_hz: u32,
    /// APB2 peripheral clock frequency in hertz.
    pub pclk2_hz: u32,
    /// USB clock domain frequency in hertz.
    pub usb_hz: u32,
}

/// Bounded scheduler configuration declared by a target manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerProfile {
    /// Number of platform timer ticks in one preemption quantum.
    pub quantum_ticks: u32,
}

/// SRAM regions declared by a board manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryProfile {
    /// Start and size of the kernel flash image region.
    pub flash: TargetMemoryRegion,
    /// Start of the kernel-reserved region.
    pub kernel_origin: u32,
    /// Size of the kernel-reserved region in bytes.
    pub kernel_length: u32,
    /// Start of the application region.
    pub application_origin: u32,
    /// Size of the application region in bytes.
    pub application_length: u32,
    /// Start of the runtime and stack region.
    pub runtime_origin: u32,
    /// Size of the runtime and stack region in bytes.
    pub runtime_length: u32,
    /// DMA-visible SRAM region reserved for transport buffers.
    pub dma: TargetMemoryRegion,
    /// Optional core-coupled memory region for privileged runtime state.
    pub ccm: Option<TargetMemoryRegion>,
    /// Optional code/data split for a future isolated application ABI.
    pub isolation: Option<IsolationMemoryProfile>,
}

/// A physical memory region declared by a target manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetMemoryRegion {
    /// Start address of the region.
    pub origin: u32,
    /// Size of the region in bytes.
    pub length: u32,
}

/// Application code and data boundaries used by the planned isolated ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsolationMemoryProfile {
    /// Start of the ordinary peripheral register region, when declared.
    pub peripheral_origin: Option<u32>,
    /// Size of the ordinary peripheral register region, when declared.
    pub peripheral_length: Option<u32>,
    /// Address reserved by the target for a deterministic BusFault fixture.
    pub bus_fault_origin: Option<u32>,
    /// Minimum aligned range declared for the BusFault fixture.
    pub bus_fault_length: Option<u32>,
    /// Ordered application slots declared by the target manifest.
    pub slots: &'static [IsolationSlot],
}

impl IsolationMemoryProfile {
    /// Returns the manifest's first slot used by the current single-app ABI.
    pub const fn active_slot(self) -> Option<IsolationSlot> {
        if self.slots.is_empty() {
            None
        } else {
            Some(self.slots[0])
        }
    }
}

/// A manifest-owned application code/data slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsolationSlot {
    /// Stable target-manifest identifier for package selection.
    pub id: u8,
    /// Stable slot name used by target-aware tooling.
    pub name: &'static str,
    /// Start of the slot's executable code region.
    pub code_origin: u32,
    /// Size of the slot's executable code region in bytes.
    pub code_length: u32,
    /// Start of the slot's writable data and PSP region.
    pub data_origin: u32,
    /// Size of the slot's writable data and PSP region in bytes.
    pub data_length: u32,
    /// PSP stack reservation inside the slot's data region.
    pub stack_length: u32,
}

/// A named GPIO pin declared by a board manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PinProfile {
    /// GPIO port name, for example `PB`.
    pub port: &'static str,
    /// GPIO pin number.
    pub number: u8,
    /// Alternate-function number, or zero for a GPIO mode.
    pub alternate_function: u8,
    /// Whether a high output means logically active.
    pub active_high: bool,
}

/// USB data-pin metadata declared by a board manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsbProfile {
    /// USB controller name.
    pub controller: &'static str,
    /// USB D- pin.
    pub dm: PinProfile,
    /// USB D+ pin.
    pub dp: PinProfile,
}

/// Storage bus metadata declared by a board manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageProfile {
    /// Storage controller name.
    pub controller: &'static str,
    /// Number of data lines used by the storage bus.
    pub bus_width: u8,
    /// Clock pin.
    pub clock: PinProfile,
    /// Command pin.
    pub command: PinProfile,
    /// Data pins in bus order.
    pub data: [PinProfile; 4],
}

include!(concat!(env!("OUT_DIR"), "/target_profiles.rs"));

/// Finds a target profile by its stable manifest name.
pub fn find_target(name: &str) -> Option<&'static TargetProfile> {
    SUPPORTED_TARGETS.iter().find(|target| target.name == name)
}

/// Finds any declared board profile, including profiles pending application support.
pub fn find_board(name: &str) -> Option<&'static TargetProfile> {
    ALL_TARGETS.iter().find(|target| target.name == name)
}

/// Finds a declared application target by its AMRN target identifier.
pub fn find_by_amrn_target_id(target_id: u8) -> Option<&'static TargetProfile> {
    ALL_TARGETS
        .iter()
        .find(|target| target.amrn_target_id == target_id && target.application_supported)
}

#[cfg(test)]
mod tests {
    use super::{DfuProfile, SUPPORTED_TARGETS, find_board};

    #[test]
    fn exposes_manifest_metadata() {
        assert_eq!(SUPPORTED_TARGETS.len(), 1);
        assert_eq!(SUPPORTED_TARGETS[0].scheduler.unwrap().quantum_ticks, 1);
        assert_eq!(SUPPORTED_TARGETS[0].name, "f405");
        assert_eq!(SUPPORTED_TARGETS[0].backend, "stm32f405");
        assert_eq!(SUPPORTED_TARGETS[0].status_led.port, "PB");
        assert!(SUPPORTED_TARGETS[0].capabilities.storage);
        assert!(SUPPORTED_TARGETS[0].capabilities.usb_console);
        assert!(SUPPORTED_TARGETS[0].capabilities.mpu);
        assert!(SUPPORTED_TARGETS[0].capabilities.relocation);
        assert_eq!(
            SUPPORTED_TARGETS[0].dfu,
            Some(DfuProfile {
                vendor_id: 0x0483,
                product_id: 0xDF11,
                address: 0x0800_0000,
                alternate: 0,
                leave: true,
            })
        );
        assert_eq!(
            SUPPORTED_TARGETS[0]
                .storage
                .as_ref()
                .map(|storage| storage.bus_width),
            Some(4)
        );
        assert!(find_board("f411").is_some());
        let isolation = SUPPORTED_TARGETS[0]
            .memory
            .isolation
            .expect("isolation metadata");
        assert_eq!(isolation.slots.len(), 2);
        assert_eq!(isolation.slots[0].name, "slot0");
        assert_eq!(isolation.slots[1].name, "slot1");
        assert_eq!(isolation.slots[0].code_origin, 0x2000_8000);
        assert_eq!(isolation.active_slot(), Some(isolation.slots[0]));
    }
}
