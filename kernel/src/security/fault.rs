//! Kernel-owned fault context for the planned isolated application boundary.
//!
//! This module defines the bounded record and recovery path for feature-gated
//! application fault handlers. ABI v2 remains unchanged.

use crate::logging;
use dali::svc::ExceptionFrame;

const EXC_RETURN_SIGNATURE_MASK: u32 = 0xFF00_0000;
const EXC_RETURN_SIGNATURE: u32 = 0xFF00_0000;
const EXC_RETURN_THREAD_MODE: u32 = 1 << 3;
const EXC_RETURN_PSP: u32 = 1 << 2;
const EXC_RETURN_BASIC_FRAME: u32 = 1 << 4;
const MEMMANAGE_ADDRESS_VALID: u32 = 1 << 7;
const BUSFAULT_ADDRESS_VALID: u32 = 1 << 15;
const USAGEFAULT_INVALID_PC: u32 = 1 << 18;
const BUSFAULT_STATUS_MASK: u32 = 0x0000_FF00;

/// Fault sources that must terminate an isolated application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultKind {
    /// The MPU rejected an access.
    MemManage,
    /// The processor reported an instruction or data bus fault.
    BusFault,
    /// The processor reported invalid execution state or instruction use.
    UsageFault,
    /// A fault occurred while entering another exception, before its handler
    /// could receive a valid application frame.
    HardFault,
    /// The exception return value did not identify the expected app context.
    InvalidExceptionReturn,
}

/// Bounded context captured before an isolated application is terminated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FaultRecord {
    /// Fault source.
    pub(crate) kind: FaultKind,
    /// Stacked program counter, when a valid basic frame was available.
    pub(crate) stacked_pc: Option<u32>,
    /// Stacked link register, when a valid basic frame was available.
    pub(crate) stacked_lr: Option<u32>,
    /// Fault address, when the architecture reported one.
    pub(crate) fault_address: Option<u32>,
    /// Architecture status bits associated with the fault.
    pub(crate) status: u32,
}

impl FaultRecord {
    /// Creates a record for a fault without a validated application frame.
    pub(crate) const fn without_frame(kind: FaultKind, status: u32) -> Self {
        Self {
            kind,
            stacked_pc: None,
            stacked_lr: None,
            fault_address: None,
            status,
        }
    }
}

/// Reports a bounded fault record without attempting recovery.
pub(crate) fn report(record: FaultRecord) {
    logging::error(
        logging::SECURITY_SUBSYSTEM,
        format_args!(
            "[SECURITY][FAULT] kind={:?} status=0x{:08X} pc={:?} lr={:?} address={:?}",
            record.kind, record.status, record.stacked_pc, record.stacked_lr, record.fault_address
        ),
    );
}

#[unsafe(export_name = "MemoryManagement")]
#[unsafe(naked)]
unsafe extern "C" fn memory_management_handler() {
    // SAFETY: The wrapper selects the hardware-stacked frame and preserves the
    // exception return value while transferring control to the kernel handler.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b {handler}",
        handler = sym handle_memory_management,
    );
}

#[unsafe(export_name = "BusFault")]
#[unsafe(naked)]
unsafe extern "C" fn bus_fault_handler() {
    // SAFETY: The wrapper selects the hardware-stacked frame and preserves the
    // exception return value while transferring control to the kernel handler.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b {handler}",
        handler = sym handle_bus_fault,
    );
}

#[unsafe(export_name = "UsageFault")]
#[unsafe(naked)]
unsafe extern "C" fn usage_fault_handler() {
    // SAFETY: The wrapper selects the hardware-stacked frame and preserves the
    // exception return value while transferring control to the kernel handler.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b {handler}",
        handler = sym handle_usage_fault,
    );
}

#[unsafe(export_name = "HardFault")]
#[unsafe(naked)]
unsafe extern "C" fn hard_fault_handler() {
    // SAFETY: The wrapper preserves only the architectural exception-return
    // value. The Rust handler validates it before reading either stack pointer.
    core::arch::naked_asm!(
        "mov r0, lr",
        "b {handler}",
        handler = sym handle_hard_fault,
    );
}

extern "C" fn handle_memory_management(frame_address: u32, exception_return: u32) -> ! {
    handle_with_frame(FaultKind::MemManage, frame_address, exception_return)
}

extern "C" fn handle_bus_fault(frame_address: u32, exception_return: u32) -> ! {
    handle_with_frame(FaultKind::BusFault, frame_address, exception_return)
}

extern "C" fn handle_usage_fault(frame_address: u32, exception_return: u32) -> ! {
    handle_with_frame(FaultKind::UsageFault, frame_address, exception_return)
}

extern "C" fn handle_hard_fault(exception_return: u32) -> ! {
    let status = unsafe {
        // SAFETY: Cortex-M4 exposes SCB at this architectural address and the
        // privileged fault handler owns this diagnostic read.
        (&*cortex_m::peripheral::SCB::PTR).cfsr.read()
    };
    let kind = if status & BUSFAULT_STATUS_MASK != 0 {
        FaultKind::BusFault
    } else {
        FaultKind::HardFault
    };
    let frame_address = application_frame_address(exception_return);
    handle_with_frame(kind, frame_address, exception_return)
}

fn application_frame_address(exception_return: u32) -> u32 {
    if !valid_application_exception_return(exception_return) {
        return 0;
    }
    if exception_return & EXC_RETURN_PSP != 0 {
        cortex_m::register::psp::read()
    } else {
        cortex_m::register::msp::read()
    }
}

fn handle_with_frame(kind: FaultKind, frame_address: u32, exception_return: u32) -> ! {
    let (status, fault_address) = read_status(kind);
    let frame = if kind == FaultKind::UsageFault && status & USAGEFAULT_INVALID_PC != 0 {
        None
    } else {
        read_application_frame(frame_address, exception_return)
    };
    let record = match frame {
        Some(frame) => FaultRecord {
            kind,
            stacked_pc: Some(frame.pc),
            stacked_lr: Some(frame.lr),
            fault_address,
            status,
        },
        None => FaultRecord {
            kind,
            stacked_pc: None,
            stacked_lr: None,
            fault_address,
            status,
        },
    };
    report(record);
    super::launch::recover()
}

fn read_status(kind: FaultKind) -> (u32, Option<u32>) {
    unsafe {
        // SAFETY: The SCB register block is an architectural MMIO region, and
        // these reads occur in privileged fault-handler context.
        let scb = &*cortex_m::peripheral::SCB::PTR;
        let status = scb.cfsr.read();
        let fault_address = match kind {
            FaultKind::MemManage if status & MEMMANAGE_ADDRESS_VALID != 0 => Some(scb.mmfar.read()),
            FaultKind::BusFault if status & BUSFAULT_ADDRESS_VALID != 0 => Some(scb.bfar.read()),
            _ => None,
        };
        (status, fault_address)
    }
}

fn read_application_frame(frame_address: u32, exception_return: u32) -> Option<ExceptionFrame> {
    if !valid_application_exception_return(exception_return) {
        return None;
    }
    let memory = crate::platform::MEMORY_PROFILE.isolation?;
    let frame_address = frame_address as usize;
    let frame_size = core::mem::size_of::<ExceptionFrame>();
    let data_start = memory.data_origin as usize;
    let data_end = data_start.checked_add(memory.data_length as usize)?;
    let frame_end = frame_address.checked_add(frame_size)?;
    if frame_address == 0
        || !frame_address.is_multiple_of(core::mem::align_of::<ExceptionFrame>())
        || frame_address < data_start
        || frame_end > data_end
    {
        return None;
    }
    unsafe {
        // SAFETY: The complete basic frame was validated inside the
        // manifest-declared application data/PSP region above.
        Some(core::ptr::read(frame_address as *const ExceptionFrame))
    }
}

fn valid_application_exception_return(value: u32) -> bool {
    value & EXC_RETURN_SIGNATURE_MASK == EXC_RETURN_SIGNATURE
        && value & EXC_RETURN_THREAD_MODE != 0
        && value & EXC_RETURN_PSP != 0
        && value & EXC_RETURN_BASIC_FRAME != 0
}
