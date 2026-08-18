//! Feature-gated SVC gateway and exception-frame validation.
//!
//! The gateway is not part of the default MVP execution path. It is compiled
//! only with `abi-current` while the privilege transition and MPU activation remain
//! separate implementation steps.

use crate::logging;
use crate::security::fault::{self, FaultKind, FaultRecord};
use dali::svc::{ExceptionFrame, ServiceId, ServiceStatus};

#[unsafe(export_name = "SVCall")]
#[unsafe(naked)]
unsafe extern "C" fn svcall_handler() {
    // SAFETY: The wrapper preserves the application register used as scratch,
    // selects the hardware-stacked frame before changing MSP, and restores the
    // original EXC_RETURN before returning to the processor.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "push {{r4, lr}}",
        "mov r4, lr",
        "mov r1, r4",
        "bl {handler}",
        "pop {{r4, lr}}",
        "bx lr",
        handler = sym handle_svc,
    );
}

fn handle_svc(frame_address: u32, exception_return: u32) {
    if !valid_exception_return(exception_return) {
        fault::report(FaultRecord::without_frame(
            FaultKind::InvalidExceptionReturn,
            exception_return,
        ));
        return;
    }
    let frame_address = frame_address as usize;
    let frame_size = core::mem::size_of::<ExceptionFrame>();
    let memory = match crate::platform::MEMORY_PROFILE.isolation {
        Some(memory) => memory,
        None => return,
    };
    let frame_end = match frame_address.checked_add(frame_size) {
        Some(end) => end,
        None => return,
    };
    let Some(slot) = memory.slots.iter().copied().find(|slot| {
        frame_address >= slot.data_origin as usize
            && frame_end <= slot.data_origin.saturating_add(slot.data_length) as usize
    }) else {
        return;
    };
    if frame_address == 0 || !frame_address.is_multiple_of(core::mem::align_of::<ExceptionFrame>())
    {
        return;
    }
    let frame = unsafe {
        // SAFETY: The PSP frame address and complete frame size were checked
        // against the manifest-declared application data/PSP region above.
        &mut *(frame_address as *mut ExceptionFrame)
    };
    dispatch(frame, slot);
}

fn valid_exception_return(value: u32) -> bool {
    const EXC_RETURN_SIGNATURE_MASK: u32 = 0xFF00_0000;
    const EXC_RETURN_SIGNATURE: u32 = 0xFF00_0000;
    const THREAD_MODE_FLAG: u32 = 1 << 3;
    const PSP_FLAG: u32 = 1 << 2;
    const BASIC_FRAME_FLAG: u32 = 1 << 4;

    value & EXC_RETURN_SIGNATURE_MASK == EXC_RETURN_SIGNATURE
        && value & THREAD_MODE_FLAG != 0
        && value & PSP_FLAG != 0
        && value & BASIC_FRAME_FLAG != 0
}

fn dispatch(frame: &mut ExceptionFrame, slot: dali_targets::IsolationSlot) {
    let status = match ServiceId::from_raw(frame.r0) {
        Some(ServiceId::Log) => dispatch_log(frame, slot),
        #[cfg(feature = "abi-test-fixtures")]
        None if frame.r0 == dali::svc::TEST_INVALID_PSP_SERVICE => dispatch_invalid_psp(slot),
        #[cfg(feature = "abi-test-fixtures")]
        None if frame.r0 == dali::svc::TEST_NO_FRAME_HARDFAULT_SERVICE => {
            dispatch_no_frame_hardfault(slot)
        }
        None => ServiceStatus::rejected(),
    };
    frame.r0 = status.0;
}

#[cfg(feature = "abi-test-fixtures")]
fn dispatch_invalid_psp(slot: dali_targets::IsolationSlot) -> ServiceStatus {
    const INVALID_PSP_OFFSET: u32 = 4;
    let Some(invalid_psp) = slot.data_origin.checked_add(INVALID_PSP_OFFSET) else {
        return ServiceStatus::rejected();
    };
    unsafe {
        // SAFETY: This path is compiled only for the non-production fixture
        // kernel. The deliberately invalid value tests exception-entry fault
        // handling; production builds do not expose this service.
        cortex_m::register::psp::write(invalid_psp);
    }
    ServiceStatus::accepted()
}

#[cfg(feature = "abi-test-fixtures")]
fn dispatch_no_frame_hardfault(slot: dali_targets::IsolationSlot) -> ServiceStatus {
    const USAGEFAULT_ENABLE_BIT: u32 = 1 << 18;
    // This path is compiled only for the non-production fixture kernel.
    // Disabling UsageFault forces the deliberate INVPC condition to escalate
    // through the HardFault entry under test.
    let shcsr = super::scb::read_shcsr();
    super::scb::write_shcsr(shcsr & !USAGEFAULT_ENABLE_BIT);
    dispatch_invalid_psp(slot)
}

fn dispatch_log(frame: &ExceptionFrame, slot: dali_targets::IsolationSlot) -> ServiceStatus {
    let message = frame.r1 as usize;
    let length = frame.r2 as usize;
    if length > dali::MAX_LOG_MESSAGE_BYTES || !contains(slot, message, length) {
        return ServiceStatus::rejected();
    }

    let bytes = unsafe {
        // SAFETY: `contains` checked that the non-null pointer and bounded length
        // are fully inside one manifest-declared application code/data region.
        core::slice::from_raw_parts(message as *const u8, length)
    };
    let Ok(message) = core::str::from_utf8(bytes) else {
        return ServiceStatus::rejected();
    };
    logging::info(logging::APPLICATION_SUBSYSTEM, format_args!("{}", message));
    ServiceStatus::accepted()
}

fn contains(slot: dali_targets::IsolationSlot, start: usize, length: usize) -> bool {
    if start > u32::MAX as usize || length > u32::MAX as usize {
        return false;
    }
    dali::svc::contains_range(
        start as u32,
        length as u32,
        slot.code_origin,
        slot.code_length,
    ) || dali::svc::contains_range(
        start as u32,
        length as u32,
        slot.data_origin,
        slot.data_length,
    )
}
