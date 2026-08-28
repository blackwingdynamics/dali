//! Retained, bounded fault evidence for the next kernel boot.

use core::cell::UnsafeCell;

use dali_sdk::svc::ExceptionFrame;

use super::FaultKind;

/// Magic identifying a retained fault-capture record.
const MAGIC: u32 = 0x4441_4643;
/// Version of the retained fault-capture layout.
const VERSION: u16 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
/// Reset-persistent fault record stored in the linker-declared section.
struct Capture {
    /// Record validity marker.
    magic: u32,
    /// Persistent record layout version.
    version: u16,
    /// Encoded fault kind.
    kind: u8,
    /// Reserved layout byte.
    reserved: u8,
    /// Architectural exception-return value.
    exception_return: u32,
    /// Address of the captured stack frame.
    frame_address: u32,
    /// Captured exception frame.
    frame: ExceptionFrame,
    /// Configurable fault-status register value.
    cfsr: u32,
    /// Hard-fault status register value.
    hfsr: u32,
    /// Memory-management fault address.
    mmfar: u32,
    /// Bus-fault address.
    bfar: u32,
    /// Integrity checksum for the record.
    checksum: u32,
}

impl Capture {
    /// Creates an empty invalid capture with the current layout version.
    const fn empty() -> Self {
        Self {
            magic: 0,
            version: VERSION,
            kind: 0,
            reserved: 0,
            exception_return: 0,
            frame_address: 0,
            frame: ExceptionFrame {
                r0: 0,
                r1: 0,
                r2: 0,
                r3: 0,
                r12: 0,
                lr: 0,
                pc: 0,
                xpsr: 0,
            },
            cfsr: 0,
            hfsr: 0,
            mmfar: 0,
            bfar: 0,
            checksum: 0,
        }
    }

    /// Computes the integrity checksum excluding the stored checksum field.
    fn checksum(&self) -> u32 {
        let mut value = u32::from(self.version) ^ u32::from(self.kind);
        value ^= self.exception_return ^ self.frame_address;
        value ^= self.frame.r0 ^ self.frame.r1 ^ self.frame.r2 ^ self.frame.r3;
        value ^= self.frame.r12 ^ self.frame.lr ^ self.frame.pc ^ self.frame.xpsr;
        value ^= self.cfsr ^ self.hfsr ^ self.mmfar ^ self.bfar;
        value.rotate_left(13) ^ 0xA5A5_5A5A
    }
}

/// Interior-mutable wrapper used for the reset-persistent capture.
struct Retained(UnsafeCell<Capture>);

// SAFETY: A fault handler is the sole writer, and boot reads the record before
// normal runtime execution begins.
unsafe impl Sync for Retained {}

#[used]
#[unsafe(link_section = ".fault_capture")]
/// Persistent capture storage retained across software reset.
static RETAINED: Retained = Retained(UnsafeCell::new(Capture::empty()));

unsafe extern "C" {
    static _stack_start: u8;
    static _stack_end: u8;
}

/// Decoded fault evidence read from reset-persistent storage.
#[derive(Clone, Copy)]
pub(crate) struct Evidence {
    /// Decoded fault kind.
    pub(crate) kind: FaultKind,
    /// Architectural exception-return value.
    pub(crate) exception_return: u32,
    /// Address of the captured stack frame.
    pub(crate) frame_address: u32,
    /// Captured exception frame.
    pub(crate) frame: ExceptionFrame,
    /// Configurable fault-status register value.
    pub(crate) cfsr: u32,
    /// Hard-fault status register value.
    pub(crate) hfsr: u32,
    /// Memory-management fault address.
    pub(crate) mmfar: u32,
    /// Bus-fault address.
    pub(crate) bfar: u32,
}

/// Stores a bounded fault capture for inspection after reboot.
pub(crate) fn capture(
    kind: FaultKind,
    exception_return: u32,
    frame_address: u32,
    cfsr: u32,
    hfsr: u32,
    mmfar: u32,
    bfar: u32,
) {
    let frame_address = raw_frame_address(frame_address, exception_return);
    let frame = read_kernel_frame(frame_address).unwrap_or_default();
    let mut capture = Capture::empty();
    capture.version = VERSION;
    capture.kind = kind_code(kind);
    capture.exception_return = exception_return;
    capture.frame_address = frame_address;
    capture.frame = frame;
    capture.cfsr = cfsr;
    capture.hfsr = hfsr;
    capture.mmfar = mmfar;
    capture.bfar = bfar;
    capture.checksum = capture.checksum();

    unsafe {
        // SAFETY: RETAINED is a dedicated, aligned, kernel-owned fault slot.
        // The magic is written last so an interrupted capture is rejected.
        let destination = RETAINED.0.get();
        core::ptr::write_volatile(destination, capture);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*destination).magic), MAGIC);
    }
}

/// Validates and consumes the retained fault capture.
pub(crate) fn take() -> Option<Evidence> {
    let capture = unsafe {
        // SAFETY: Boot reads the retained slot before handing execution to the
        // runtime; volatile access preserves evidence across software reset.
        core::ptr::read_volatile(RETAINED.0.get())
    };
    if capture.magic != MAGIC
        || capture.version != VERSION
        || capture.checksum != capture.checksum()
    {
        return None;
    }
    unsafe {
        // SAFETY: Invalidating the dedicated retained slot is ordered after its
        // contents have been copied into the boot-local value above.
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*RETAINED.0.get()).magic), 0);
    }
    Some(Evidence {
        kind: kind_from_code(capture.kind),
        exception_return: capture.exception_return,
        frame_address: capture.frame_address,
        frame: capture.frame,
        cfsr: capture.cfsr,
        hfsr: capture.hfsr,
        mmfar: capture.mmfar,
        bfar: capture.bfar,
    })
}

/// Computes the raw frame address selected by an exception return value.
fn raw_frame_address(address: u32, exception_return: u32) -> u32 {
    if address != 0 {
        return address;
    }
    if exception_return & 0xFF00_0000 != 0xFF00_0000 {
        return 0;
    }
    if exception_return & (1 << 2) != 0 {
        cortex_m::register::psp::read()
    } else {
        cortex_m::register::msp::read()
    }
}

/// Reads a frame known to reside on the kernel stack.
fn read_kernel_frame(address: u32) -> Option<ExceptionFrame> {
    let start = core::ptr::addr_of!(_stack_end) as usize;
    let end = core::ptr::addr_of!(_stack_start) as usize;
    let address = address as usize;
    let frame_end = address.checked_add(core::mem::size_of::<ExceptionFrame>())?;
    if address < start
        || frame_end > end
        || !address.is_multiple_of(core::mem::align_of::<ExceptionFrame>())
    {
        return None;
    }
    unsafe {
        // SAFETY: The complete frame lies inside the linker-declared kernel
        // stack range and is read only while handling the exception.
        Some(core::ptr::read_volatile(address as *const ExceptionFrame))
    }
}

/// Encodes a fault kind for the persistent record.
fn kind_code(kind: FaultKind) -> u8 {
    match kind {
        FaultKind::MemManage => 1,
        FaultKind::BusFault => 2,
        FaultKind::UsageFault => 3,
        FaultKind::HardFault => 4,
        FaultKind::InvalidExceptionReturn => 5,
    }
}

/// Decodes a persisted fault kind, defaulting to hard fault.
fn kind_from_code(code: u8) -> FaultKind {
    match code {
        1 => FaultKind::MemManage,
        2 => FaultKind::BusFault,
        3 => FaultKind::UsageFault,
        5 => FaultKind::InvalidExceptionReturn,
        _ => FaultKind::HardFault,
    }
}
