//! Feature-gated SVC gateway and exception-frame validation.
//!
//! The gateway is not part of the default MVP execution path. It is compiled
//! only with `abi-v3` while the privilege transition and MPU activation remain
//! separate implementation steps.

use cortex_m_rt::exception;
use dali::svc::{ExceptionFrame, ServiceId, ServiceStatus};
use dali_targets::IsolationMemoryProfile;

use crate::logging;
use crate::security::fault::{self, FaultKind, FaultRecord};

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
    let memory = match dali_targets::TARGET_F405.memory.isolation {
        Some(memory) => memory,
        None => return,
    };
    let data_start = memory.data_origin as usize;
    let data_end = match data_start.checked_add(memory.data_length as usize) {
        Some(end) => end,
        None => return,
    };
    let frame_end = match frame_address.checked_add(frame_size) {
        Some(end) => end,
        None => return,
    };
    if frame_address == 0
        || !frame_address.is_multiple_of(core::mem::align_of::<ExceptionFrame>())
        || frame_address < data_start
        || frame_end > data_end
    {
        return;
    }
    let frame = unsafe {
        // SAFETY: The PSP frame address and complete frame size were checked
        // against the manifest-declared application data/PSP region above.
        &mut *(frame_address as *mut ExceptionFrame)
    };
    dispatch(frame, memory);
}

#[exception]
fn MemoryManagement() {
    fault::handle(FaultKind::MemManage);
}

#[exception]
fn BusFault() {
    fault::handle(FaultKind::BusFault);
}

#[exception]
fn UsageFault() {
    fault::handle(FaultKind::UsageFault);
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

fn dispatch(frame: &mut ExceptionFrame, memory: IsolationMemoryProfile) {
    let status = match ServiceId::from_raw(frame.r0) {
        Some(ServiceId::Log) => dispatch_log(frame, memory),
        None => ServiceStatus::rejected(),
    };
    frame.r0 = status.0;
}

fn dispatch_log(frame: &ExceptionFrame, memory: IsolationMemoryProfile) -> ServiceStatus {
    let message = frame.r1 as usize;
    let length = frame.r2 as usize;
    if length > dali::MAX_LOG_MESSAGE_BYTES || !contains(memory, message, length) {
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

fn contains(memory: IsolationMemoryProfile, start: usize, length: usize) -> bool {
    if start > u32::MAX as usize || length > u32::MAX as usize {
        return false;
    }
    dali::svc::contains_range(
        start as u32,
        length as u32,
        memory.code_origin,
        memory.code_length,
    ) || dali::svc::contains_range(
        start as u32,
        length as u32,
        memory.data_origin,
        memory.data_length,
    )
}
