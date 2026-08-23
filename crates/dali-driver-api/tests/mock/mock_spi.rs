use dali_driver_api::{BoundedTimeout, DriverError, DriverResult, Duration, SpiTransfer};

pub struct MockSpi<const CAPACITY: usize> {
    pub max_timeout: Duration,
    pub response: [u8; CAPACITY],
    pub response_len: usize,
    pub connected: bool,
    pub would_block: bool,
    pub nack: bool,
    pub arbitration_lost: bool,
}

impl<const CAPACITY: usize> MockSpi<CAPACITY> {
    pub const fn new(max_timeout: Duration) -> Self {
        Self {
            max_timeout,
            response: [0; CAPACITY],
            response_len: 0,
            connected: true,
            would_block: false,
            nack: false,
            arbitration_lost: false,
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

impl<const CAPACITY: usize> SpiTransfer for MockSpi<CAPACITY> {
    fn transfer(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.connected {
            return Err(DriverError::Disconnected);
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
