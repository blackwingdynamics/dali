//! Bounded I2C register transactions for the F405 adapter.

use super::F405I2c;
use dali_driver_api::{DriverError, DriverResult, Duration, I2cAddress};

impl F405I2c {
    fn consume(budget: &mut u32) -> DriverResult<()> {
        if *budget == 0 {
            Err(DriverError::Timeout)
        } else {
            *budget -= 1;
            Ok(())
        }
    }

    fn check_error(&mut self) -> DriverResult<()> {
        let status = self.peripheral.sr1.read();
        if status.timeout().bit_is_set() {
            self.peripheral
                .sr1
                .modify(|_, writer| writer.timeout().clear_bit());
            return Err(DriverError::Timeout);
        }
        if status.af().bit_is_set() {
            self.peripheral
                .sr1
                .modify(|_, writer| writer.af().clear_bit());
            return Err(DriverError::Nack);
        }
        if status.arlo().bit_is_set() {
            self.peripheral
                .sr1
                .modify(|_, writer| writer.arlo().clear_bit());
            return Err(DriverError::ArbitrationLost);
        }
        if status.berr().bit_is_set() {
            self.peripheral
                .sr1
                .modify(|_, writer| writer.berr().clear_bit());
            return Err(DriverError::BusError);
        }
        if status.ovr().bit_is_set() {
            self.peripheral
                .sr1
                .modify(|_, writer| writer.ovr().clear_bit());
            return Err(DriverError::BusError);
        }
        Ok(())
    }

    fn wait_bus_idle(&mut self, budget: &mut u32) -> DriverResult<()> {
        while self.peripheral.sr2.read().busy().bit_is_set() {
            self.check_error()?;
            Self::consume(budget)?;
        }
        Ok(())
    }

    fn start(&mut self, budget: &mut u32) -> DriverResult<()> {
        self.wait_bus_idle(budget)?;
        self.peripheral
            .cr1
            .modify(|_, writer| writer.start().set_bit());
        while self.peripheral.sr1.read().sb().bit_is_clear() {
            self.check_error()?;
            Self::consume(budget)?;
        }
        Ok(())
    }

    fn repeated_start(&mut self, budget: &mut u32) -> DriverResult<()> {
        self.peripheral
            .cr1
            .modify(|_, writer| writer.start().set_bit());
        while self.peripheral.sr1.read().sb().bit_is_clear() {
            self.check_error()?;
            Self::consume(budget)?;
        }
        Ok(())
    }

    fn send_address(
        &mut self,
        address: I2cAddress,
        read: bool,
        budget: &mut u32,
    ) -> DriverResult<()> {
        let value = (u32::from(address.value()) << 1) | u32::from(read);
        // SAFETY: The address is a validated logical I2C address value and DR
        // accepts the corresponding 8-bit address phase encoding.
        self.peripheral
            .dr
            .write(|writer| unsafe { writer.bits(value) });
        while self.peripheral.sr1.read().addr().bit_is_clear() {
            self.check_error()?;
            Self::consume(budget)?;
        }
        self.peripheral.sr2.read();
        Ok(())
    }

    fn send_byte(&mut self, byte: u8, budget: &mut u32) -> DriverResult<()> {
        while self.peripheral.sr1.read().tx_e().bit_is_clear() {
            self.check_error()?;
            Self::consume(budget)?;
        }
        // SAFETY: DR is the PAC-owned I2C transmit register and receives one
        // caller-provided byte after the bounded TXE check.
        self.peripheral
            .dr
            .write(|writer| unsafe { writer.bits(u32::from(byte)) });
        Ok(())
    }

    fn finish_write(&mut self, budget: &mut u32) -> DriverResult<()> {
        while self.peripheral.sr1.read().btf().bit_is_clear() {
            self.check_error()?;
            Self::consume(budget)?;
        }
        self.peripheral
            .cr1
            .modify(|_, writer| writer.stop().set_bit());
        Ok(())
    }

    fn write_phase(
        &mut self,
        address: I2cAddress,
        bytes: &[u8],
        budget: &mut u32,
    ) -> DriverResult<()> {
        self.start(budget)?;
        self.send_address(address, false, budget)?;
        for byte in bytes {
            self.send_byte(*byte, budget)?;
        }
        Ok(())
    }

    fn read_phase(
        &mut self,
        address: I2cAddress,
        buffer: &mut [u8],
        budget: &mut u32,
        repeated: bool,
    ) -> DriverResult<()> {
        if buffer.is_empty() {
            return Ok(());
        }
        self.peripheral
            .cr1
            .modify(|_, writer| writer.ack().set_bit());
        if repeated {
            self.repeated_start(budget)?;
        } else {
            self.start(budget)?;
        }
        self.send_address(address, true, budget)?;
        if buffer.len() == 1 {
            self.peripheral
                .cr1
                .modify(|_, writer| writer.ack().clear_bit());
        }
        let length = buffer.len();
        for (index, byte) in buffer.iter_mut().enumerate() {
            while self.peripheral.sr1.read().rx_ne().bit_is_clear() {
                self.check_error()?;
                Self::consume(budget)?;
            }
            if index + 1 == length {
                self.peripheral
                    .cr1
                    .modify(|_, writer| writer.stop().set_bit());
            }
            *byte = self.peripheral.dr.read().bits() as u8;
            if index + 1 == length {
                self.peripheral
                    .cr1
                    .modify(|_, writer| writer.ack().set_bit());
            }
        }
        Ok(())
    }

    fn finish(&mut self, result: DriverResult<()>) -> DriverResult<()> {
        if matches!(result, Err(DriverError::Timeout | DriverError::BusError)) {
            let recovery = self.recover();
            if recovery.is_err() {
                return recovery;
            }
        }
        result
    }

    pub(super) fn execute_write(
        &mut self,
        address: I2cAddress,
        bytes: &[u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        let mut budget = self.timeout.poll_budget(timeout)?;
        let result = self.write_phase(address, bytes, &mut budget);
        let result = result.and_then(|_| self.finish_write(&mut budget));
        self.finish(result)
    }

    pub(super) fn execute_read(
        &mut self,
        address: I2cAddress,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        let mut budget = self.timeout.poll_budget(timeout)?;
        let result = self.read_phase(address, buffer, &mut budget, false);
        self.finish(result)
    }

    pub(super) fn execute_write_read(
        &mut self,
        address: I2cAddress,
        write_bytes: &[u8],
        read_buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        let mut budget = self.timeout.poll_budget(timeout)?;
        let result = self.write_phase(address, write_bytes, &mut budget);
        let result = if read_buffer.is_empty() {
            result.and_then(|_| self.finish_write(&mut budget))
        } else {
            result.and_then(|_| self.read_phase(address, read_buffer, &mut budget, true))
        };
        self.finish(result)
    }
}
