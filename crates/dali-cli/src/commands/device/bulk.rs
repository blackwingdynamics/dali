//! Host discovery for the dedicated USB bulk installer interface.

use dali_device::{Capability, DeviceRecord, State, Transport};
use nusb::MaybeFuture;
use nusb::transfer::{Bulk, In, Out};

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

/// Claims one discovered installer interface and verifies its bulk endpoints.
pub(super) fn verify(record: &DeviceRecord) -> Result<(), String> {
    let vendor_id = parse_id(record.vendor.as_deref().ok_or("installer vendor missing")?)?;
    let target = record.target.as_deref().ok_or("installer target missing")?;
    let profile =
        dali_targets::find_board(target).ok_or_else(|| format!("target '{target}' missing"))?;
    let serial = record.serial.as_deref();
    let mut devices = nusb::list_devices()
        .wait()
        .map_err(|error| format!("bulk discovery failed: {error}"))?;
    let device = devices
        .find(|device| {
            device.vendor_id() == vendor_id
                && device.product_id() == profile.usb.product_id
                && device.serial_number() == serial
        })
        .ok_or_else(|| format!("installer '{}' is no longer connected", record.id))?;
    let device = device
        .open()
        .wait()
        .map_err(|error| format!("installer open failed: {error}"))?;
    let interface_number = interface_number(record)?;
    let interface = device
        .claim_interface(interface_number)
        .wait()
        .map_err(|error| format!("installer interface claim failed: {error}"))?;
    let descriptor = interface
        .descriptor()
        .ok_or("installer interface descriptor missing")?;
    let mut bulk_in = None;
    let mut bulk_out = None;
    for endpoint in descriptor.endpoints() {
        if endpoint.transfer_type() != nusb::descriptors::TransferType::Bulk {
            continue;
        }
        if endpoint.address() & 0x80 == 0 {
            bulk_out = Some(endpoint.address());
        } else {
            bulk_in = Some(endpoint.address());
        }
    }
    let out_address = bulk_out.ok_or("installer bulk OUT endpoint missing")?;
    let in_address = bulk_in.ok_or("installer bulk IN endpoint missing")?;
    let _out = interface
        .endpoint::<Bulk, Out>(out_address)
        .map_err(|error| format!("installer bulk OUT open failed: {error}"))?;
    let _in = interface
        .endpoint::<Bulk, In>(in_address)
        .map_err(|error| format!("installer bulk IN open failed: {error}"))?;
    Ok(())
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

fn parse_id(value: &str) -> Result<u16, String> {
    u16::from_str_radix(value, 16).map_err(|_| format!("invalid USB vendor id '{value}'"))
}

fn interface_number(record: &DeviceRecord) -> Result<u8, String> {
    record
        .path
        .as_deref()
        .and_then(|path| path.strip_prefix("interface:"))
        .ok_or_else(|| "installer interface number missing".to_owned())?
        .parse()
        .map_err(|_| "invalid installer interface number".to_owned())
}
