//! F405 bulk USB interface used by the bounded cartridge installer.

use usb_device::{Result, class_prelude::*};

const INSTALLER_INTERFACE_CLASS: u8 = 0xFF;
const INSTALLER_INTERFACE_SUBCLASS: u8 = 0x00;
const INSTALLER_INTERFACE_PROTOCOL: u8 = 0x00;
const INSTALLER_INTERFACE_NAME: &str = "Dali Cartridge Installer";

/// Dedicated bulk interface that does not consume the three endpoints needed
/// by a second CDC-ACM class.
pub(crate) struct InstallerBulkClass<'a, B: UsbBus> {
    interface: InterfaceNumber,
    interface_string: StringIndex,
    read_endpoint: EndpointOut<'a, B>,
    write_endpoint: EndpointIn<'a, B>,
}

impl<'a, B: UsbBus> InstallerBulkClass<'a, B> {
    /// Allocates one installer interface with bounded full-speed bulk packets.
    pub(crate) fn new(alloc: &'a UsbBusAllocator<B>) -> Self {
        Self {
            interface: alloc.interface(),
            interface_string: alloc.string(),
            read_endpoint: alloc.bulk(64),
            write_endpoint: alloc.bulk(64),
        }
    }

    /// Reads one bounded packet received from the installer host.
    pub(crate) fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.read_endpoint.read(buffer)
    }
}

impl<B: UsbBus> UsbClass<B> for InstallerBulkClass<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface_alt(
            self.interface,
            0,
            INSTALLER_INTERFACE_CLASS,
            INSTALLER_INTERFACE_SUBCLASS,
            INSTALLER_INTERFACE_PROTOCOL,
            Some(self.interface_string),
        )?;
        writer.endpoint(&self.read_endpoint)?;
        writer.endpoint(&self.write_endpoint)
    }

    fn get_string(&self, index: StringIndex, lang_id: LangID) -> Option<&str> {
        (lang_id == LangID::EN_US && index == self.interface_string)
            .then_some(INSTALLER_INTERFACE_NAME)
    }
}
