use super::*;

impl RawSdioReader {
    /// Configures the `configure dma` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    pub(super) fn configure_dma(registers: &pac::sdio::RegisterBlock, words: &mut [u32]) {
        let rcc = Self::rcc();
        rcc.ahb1enr.modify(|_, writer| writer.dma2en().enabled());
        rcc.ahb1rstr.modify(|_, writer| writer.dma2rst().set_bit());
        rcc.ahb1rstr
            .modify(|_, writer| writer.dma2rst().clear_bit());
        let dma = Self::dma2();
        dma.lifcr.write(|writer| {
            writer
                .ctcif3()
                .clear()
                .chtif3()
                .clear()
                .cteif3()
                .clear()
                .cdmeif3()
                .clear()
                .cfeif3()
                .clear()
        });
        let stream = &dma.st[DMA_STREAM_INDEX];
        // SAFETY: DMA channel and word-size values are named constants within
        // the STM32F405 SDIO DMA mapping and are valid for this register.
        unsafe {
            stream.cr.write(|writer| {
                writer
                    .chsel()
                    .bits(DMA_CHANNEL)
                    .dir()
                    .peripheral_to_memory()
                    .psize()
                    .bits(DMA_WORD_SIZE)
                    .msize()
                    .bits(DMA_WORD_SIZE)
                    .minc()
                    .set_bit()
                    .pburst()
                    .incr4()
                    .mburst()
                    .incr4()
                    .pl()
                    .high()
            });
        }
        stream
            .fcr
            .write(|writer| writer.dmdis().enabled().fth().full());
        stream
            .par
            .write(|writer| unsafe { writer.pa().bits(registers.fifo.as_ptr() as u32) });
        stream
            .m0ar
            .write(|writer| unsafe { writer.m0a().bits(words.as_mut_ptr() as u32) });
        stream
            .ndtr
            .write(|writer| writer.ndt().bits(DATA_WORD_COUNT));
        stream.cr.modify(|_, writer| writer.en().set_bit());
    }

    /// Performs the `receive block` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    pub(super) fn receive_block(
        registers: &pac::sdio::RegisterBlock,
        words: &mut [u32],
    ) -> Result<(), StorageError> {
        loop {
            let status = registers.sta.read();
            status_error(&status)?;
            let flags = Self::dma2().lisr.read();
            if flags.teif3().bit() || flags.dmeif3().bit() || flags.feif3().bit() {
                return Err(StorageError::Transport);
            }
            if flags.tcif3().bit() {
                return Ok(());
            }
            if status.dtimeout().bit_is_set() {
                return Err(StorageError::Timeout);
            }
            if registers.dcount.read().datacount().bits() == 0 {
                return Self::finish_dma_tail(registers, words);
            }
            if status.rxact().bit_is_clear()
                && status.cmdact().bit_is_clear()
                && status.cmdrend().bit_is_set()
            {
                return Err(StorageError::Transport);
            }
        }
    }

    /// Finishes the `finish dma tail` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    fn finish_dma_tail(
        registers: &pac::sdio::RegisterBlock,
        words: &mut [u32],
    ) -> Result<(), StorageError> {
        let remaining = Self::dma2().st[DMA_STREAM_INDEX].ndtr.read().ndt().bits() as usize;
        if remaining == 0 {
            return Ok(());
        }
        if remaining > words.len() || !registers.sta.read().rxdavl().bit() {
            return Err(StorageError::Transport);
        }

        Self::stop_dma();
        let offset = words.len() - remaining;
        for word in &mut words[offset..] {
            *word = registers.fifo.read().bits();
        }
        Ok(())
    }

    /// Stops the `stop dma` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    pub(super) fn stop_dma() {
        let stream = &Self::dma2().st[DMA_STREAM_INDEX];
        stream.cr.modify(|_, writer| writer.en().clear_bit());
        while stream.cr.read().en().bit() {}
    }

    /// Performs the `rcc` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    fn rcc() -> &'static pac::rcc::RegisterBlock {
        // SAFETY: RCC is a singleton peripheral accessed only for DMA2 clock setup.
        unsafe { &*pac::RCC::ptr() }
    }

    /// Performs the `dma2` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    fn dma2() -> &'static pac::dma2::RegisterBlock {
        // SAFETY: DMA2 stream 3 is exclusively owned by the SDIO block reader.
        unsafe { &*pac::DMA2::ptr() }
    }
}
