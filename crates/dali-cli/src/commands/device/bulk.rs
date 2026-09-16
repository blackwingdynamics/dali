//! Host discovery for the dedicated USB bulk installer interface.

use dali_device::{Capability, DeviceRecord, State, Transport};
use nusb::MaybeFuture;

const INSTALLER_INTERFACE_CLASS: u8 = 0xFF;
const USB_ID_WIDTH: usize = 4;

/// Discovers composite USB devices exposing Dali's vendor-specific installer interface.
pub(super) fn discover() -> Result<Vec<DeviceRecord>, String> {
    let devices = nusb::list_devices()
        .wait()
        .map_err(|error| format!("bulk discovery failed: {error}"))?;
    Ok(devices
        .filter_map(|device| {
            let profile = dali_targets::ALL_TARGETS.iter().find(|profile| {
                profile.usb.vendor_id == device.vendor_id()
                    && profile.usb.product_id == device.product_id()
            })?;
            let interface = device
                .interfaces()
                .find(|interface| interface.class() == INSTALLER_INTERFACE_CLASS)?;
            let serial = device.serial_number().map(str::to_owned);
            let id = device_id(device.vendor_id(), device.product_id(), serial.as_deref());
            Some(DeviceRecord {
                id,
                transport: Transport::Bulk,
                target: Some(profile.name.to_owned()),
                vendor: Some(format!("{:04X}", device.vendor_id())),
                product: interface
                    .interface_string()
                    .or_else(|| device.product_string())
                    .map(str::to_owned),
                serial,
                path: Some(format!("interface:{}", interface.interface_number())),
                state: State::Available,
                capabilities: vec![Capability::Install],
                diagnostic: None,
            })
        })
        .collect())
}

fn device_id(vendor_id: u16, product_id: u16, serial: Option<&str>) -> String {
    let identity = serial.unwrap_or("no-serial");
    format!(
        "usb:{:0width$X}:{:0width$X}:{identity}",
        vendor_id,
        product_id,
        width = USB_ID_WIDTH,
    )
}
