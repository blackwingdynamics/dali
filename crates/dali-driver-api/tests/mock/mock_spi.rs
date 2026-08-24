use dali_driver_api::{
    BoundedTimeout, DriverError, DriverResult, Duration, SpiBusOwnership, SpiDeviceId,
    SpiDeviceSelect, SpiTransfer,
};

pub struct MockSpi<const CAPACITY: usize> {
    pub max_timeout: Duration,
    pub response: [u8; CAPACITY],
    pub response_len: usize,
    pub connected: bool,
    pub timed_out: bool,
    pub would_block: bool,
    pub nack: bool,
    pub arbitration_lost: bool,
    pub owned: bool,
    pub selected: Option<SpiDeviceId>,
}

impl<const CAPACITY: usize> MockSpi<CAPACITY> {
    pub const fn new(max_timeout: Duration) -> Self {
        Self {
            max_timeout,
            response: [0; CAPACITY],
            response_len: 0,
            connected: true,
            timed_out: false,
            would_block: false,
            nack: false,
            arbitration_lost: false,
            owned: false,
            selected: None,
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
}

impl<const CAPACITY: usize> BoundedTimeout for MockSpi<CAPACITY> {
    fn max_timeout(&self) -> Duration {
        self.max_timeout
    }
}

impl<const CAPACITY: usize> SpiBusOwnership for MockSpi<CAPACITY> {
    fn acquire(&mut self) -> DriverResult<()> {
        if self.owned {
            Err(DriverError::ResourceBusy)
        } else {
            self.owned = true;
            Ok(())
        }
    }

    fn release(&mut self) -> DriverResult<()> {
        if !self.owned || self.selected.is_some() {
            return Err(DriverError::InvalidState);
        }
        self.owned = false;
        Ok(())
    }

    fn is_owned(&self) -> bool {
        self.owned
    }
}

impl<const CAPACITY: usize> SpiDeviceSelect for MockSpi<CAPACITY> {
    fn select(&mut self, device: SpiDeviceId) -> DriverResult<()> {
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        if self.selected.is_some() {
            return Err(DriverError::ResourceBusy);
        }
        self.selected = Some(device);
        Ok(())
    }

    fn deselect(&mut self) -> DriverResult<()> {
        if self.selected.take().is_none() {
            return Err(DriverError::InvalidState);
        }
        Ok(())
    }

    fn selected(&self) -> Option<SpiDeviceId> {
        self.selected
    }
}

impl<const CAPACITY: usize> SpiTransfer for MockSpi<CAPACITY> {
    fn transfer(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.connected {
            return Err(DriverError::Disconnected);
        }
        if self.timed_out {
            return Err(DriverError::Timeout);
        }
        if self.would_block {
            return Err(DriverError::WouldBlock);
        }
        if self.nack {
            return Err(DriverError::Nack);
        }
        if self.arbitration_lost {
            return Err(DriverError::ArbitrationLost);
        }
        if buffer.len() < self.response_len {
            return Err(DriverError::InvalidBuffer);
        }
        buffer[..self.response_len].copy_from_slice(&self.response[..self.response_len]);
        Ok(())
    }
}
