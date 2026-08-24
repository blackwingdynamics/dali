use dali_driver_api::{
    BoundedTimeout, DriverError, DriverResult, Duration, SerialConfig, SerialConfigure,
    SerialOwnership, SerialRead, SerialWrite,
};

pub struct MockSerial<const CAPACITY: usize> {
    pub max_timeout: Duration,
    pub rx: [u8; CAPACITY],
    pub rx_len: usize,
    pub tx: [u8; CAPACITY],
    pub tx_len: usize,
    pub connected: bool,
    pub would_block: bool,
    pub framing_error: bool,
    pub parity_error: bool,
    pub overrun: bool,
    pub owned: bool,
    pub config: Option<SerialConfig>,
}

impl<const CAPACITY: usize> MockSerial<CAPACITY> {
    pub const fn new(max_timeout: Duration) -> Self {
        Self {
            max_timeout,
            rx: [0; CAPACITY],
            rx_len: 0,
            tx: [0; CAPACITY],
            tx_len: 0,
            connected: true,
            would_block: false,
            framing_error: false,
            parity_error: false,
            overrun: false,
            owned: false,
            config: None,
        }
    }

    pub fn queue_rx(&mut self, bytes: &[u8]) -> DriverResult<()> {
        if bytes.len() > CAPACITY {
            return Err(DriverError::InvalidBuffer);
        }
        self.rx[..bytes.len()].copy_from_slice(bytes);
        self.rx_len = bytes.len();
        Ok(())
    }

    fn check(&self, timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.connected {
            return Err(DriverError::Disconnected);
        }
        if self.would_block {
            return Err(DriverError::WouldBlock);
        }
        if self.framing_error {
            return Err(DriverError::Framing);
        }
        if self.parity_error {
            return Err(DriverError::Parity);
        }
        if self.overrun {
            return Err(DriverError::Overrun);
        }
        Ok(())
    }
}

impl<const CAPACITY: usize> BoundedTimeout for MockSerial<CAPACITY> {
    fn max_timeout(&self) -> Duration {
        self.max_timeout
    }
}

impl<const CAPACITY: usize> SerialOwnership for MockSerial<CAPACITY> {
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
}

impl<const CAPACITY: usize> SerialConfigure for MockSerial<CAPACITY> {
    fn configure(&mut self, config: SerialConfig) -> DriverResult<()> {
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        self.config = Some(config);
        Ok(())
    }
}

impl<const CAPACITY: usize> SerialRead for MockSerial<CAPACITY> {
    fn read(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<usize> {
        self.check(timeout)?;
        let count = buffer.len().min(self.rx_len);
        buffer[..count].copy_from_slice(&self.rx[..count]);
        self.rx.copy_within(count..self.rx_len, 0);
        self.rx_len -= count;
        Ok(count)
    }
}

impl<const CAPACITY: usize> SerialWrite for MockSerial<CAPACITY> {
    fn write(&mut self, buffer: &[u8], timeout: Duration) -> DriverResult<usize> {
        self.check(timeout)?;
        let remaining = CAPACITY.saturating_sub(self.tx_len);
        if buffer.len() > remaining {
            return Err(DriverError::InvalidBuffer);
        }
        let end = self.tx_len + buffer.len();
        self.tx[self.tx_len..end].copy_from_slice(buffer);
        self.tx_len = end;
        Ok(buffer.len())
    }

    fn flush(&mut self, timeout: Duration) -> DriverResult<()> {
        self.check(timeout)
    }
}
