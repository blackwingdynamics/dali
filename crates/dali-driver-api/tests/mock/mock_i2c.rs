use dali_driver_api::{BoundedTimeout, DriverError, DriverResult, Duration, I2cAddress, I2cDriver};

/// Recorded operation kind used to verify transaction sequencing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I2cOperation {
    /// A write-only transaction.
    Write,
    /// A read-only transaction.
    Read,
    /// A write followed by a repeated-start read.
    WriteRead,
}

/// Deterministic host double for the hardware-neutral I2C contract.
pub struct MockI2c<const CAPACITY: usize> {
    pub max_timeout: Duration,
    pub response: [u8; CAPACITY],
    pub response_len: usize,
    pub last_write: [u8; CAPACITY],
    pub last_write_len: usize,
    pub last_address: Option<I2cAddress>,
    pub last_operation: Option<I2cOperation>,
    pub write_read_calls: usize,
    pub connected: bool,
    pub timed_out: bool,
    pub nack: bool,
    pub arbitration_lost: bool,
    pub bus_error: bool,
    pub owned: bool,
}

impl<const CAPACITY: usize> MockI2c<CAPACITY> {
    pub const fn new(max_timeout: Duration) -> Self {
        Self {
            max_timeout,
            response: [0; CAPACITY],
            response_len: 0,
            last_write: [0; CAPACITY],
            last_write_len: 0,
            last_address: None,
            last_operation: None,
            write_read_calls: 0,
            connected: true,
            timed_out: false,
            nack: false,
            arbitration_lost: false,
            bus_error: false,
            owned: false,
        }
    }

    pub fn set_response(&mut self, response: &[u8]) -> DriverResult<()> {
        if response.len() > CAPACITY {
            return Err(DriverError::InvalidBuffer);
        }
        self.response[..response.len()].copy_from_slice(response);
        self.response_len = response.len();
        Ok(())
    }

    fn validate_operation(&self, address: I2cAddress, timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        if !self.connected {
            return Err(DriverError::Disconnected);
        }
        if self.timed_out {
            return Err(DriverError::Timeout);
        }
        if self.nack {
            return Err(DriverError::Nack);
        }
        if self.arbitration_lost {
            return Err(DriverError::ArbitrationLost);
        }
        if self.bus_error {
            return Err(DriverError::BusError);
        }
        let _ = address;
        Ok(())
    }

    fn record_write(&mut self, address: I2cAddress, bytes: &[u8]) -> DriverResult<()> {
        if bytes.len() > CAPACITY {
            return Err(DriverError::InvalidBuffer);
        }
        self.last_write[..bytes.len()].copy_from_slice(bytes);
        self.last_write_len = bytes.len();
        self.last_address = Some(address);
        Ok(())
    }

    fn copy_response(&self, buffer: &mut [u8]) -> DriverResult<()> {
        if buffer.len() < self.response_len {
            return Err(DriverError::InvalidBuffer);
        }
        buffer[..self.response_len].copy_from_slice(&self.response[..self.response_len]);
        Ok(())
    }
}

impl<const CAPACITY: usize> BoundedTimeout for MockI2c<CAPACITY> {
    fn max_timeout(&self) -> Duration {
        self.max_timeout
    }
}

impl<const CAPACITY: usize> I2cDriver for MockI2c<CAPACITY> {
    fn acquire(&mut self) -> DriverResult<()> {
        if self.owned {
            Err(DriverError::ResourceBusy)
        } else {
            self.owned = true;
            Ok(())
        }
    }

    fn release(&mut self) -> DriverResult<()> {
        if !self.owned {
            Err(DriverError::InvalidState)
        } else {
            self.owned = false;
            Ok(())
        }
    }

    fn is_owned(&self) -> bool {
        self.owned
    }

    fn write(&mut self, address: I2cAddress, bytes: &[u8], timeout: Duration) -> DriverResult<()> {
        self.validate_operation(address, timeout)?;
        self.record_write(address, bytes)?;
        self.last_operation = Some(I2cOperation::Write);
        Ok(())
    }

    fn read(
        &mut self,
        address: I2cAddress,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        self.validate_operation(address, timeout)?;
        self.copy_response(buffer)?;
        self.last_address = Some(address);
        self.last_operation = Some(I2cOperation::Read);
        Ok(())
    }

    fn write_read(
        &mut self,
        address: I2cAddress,
        write_bytes: &[u8],
        read_buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        self.validate_operation(address, timeout)?;
        self.record_write(address, write_bytes)?;
        self.copy_response(read_buffer)?;
        self.write_read_calls += 1;
        self.last_operation = Some(I2cOperation::WriteRead);
        Ok(())
    }
}
