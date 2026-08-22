struct MockReader {
    blocks: [super::Block; 2],
}

impl super::BlockReader for MockReader {
    fn read_block(
        &mut self,
        address: super::BlockAddress,
        buffer: &mut super::Block,
    ) -> Result<(), super::StorageError> {
        let source = self
            .blocks
            .get(address.value() as usize)
            .ok_or(super::StorageError::InvalidBlockAddress)?;
        buffer.copy_from_slice(source);
        Ok(())
    }

    fn block_count(&self) -> Result<u32, super::StorageError> {
        Ok(self.blocks.len() as u32)
    }
}

#[test]
fn adapts_bounded_reads_without_hardware_dependencies() {
    let reader = MockReader {
        blocks: [[0x11; super::BLOCK_SIZE], [0x22; super::BLOCK_SIZE]],
    };
    let device = super::BlockDeviceAdapter::new(reader);
    let mut blocks: [embedded_sdmmc::Block; 2] = core::array::from_fn(|_| embedded_sdmmc::Block {
        contents: [0; super::BLOCK_SIZE],
    });

    <super::BlockDeviceAdapter<MockReader> as embedded_sdmmc::BlockDevice>::read(
        &device,
        &mut blocks,
        embedded_sdmmc::BlockIdx(0),
    )
    .expect("mock read succeeds");

    assert_eq!(blocks[0].contents, [0x11; super::BLOCK_SIZE]);
    assert_eq!(blocks[1].contents, [0x22; super::BLOCK_SIZE]);
    assert_eq!(
        <super::BlockDeviceAdapter<MockReader> as embedded_sdmmc::BlockDevice>::num_blocks(
            &device,
        )
        .expect("count succeeds")
        .0,
        2
    );
}

#[test]
fn keeps_the_read_only_contract() {
    let reader = MockReader {
        blocks: [[0; super::BLOCK_SIZE]; 2],
    };
    let device = super::BlockDeviceAdapter::new(reader);
    let blocks = [embedded_sdmmc::Block {
        contents: [0; super::BLOCK_SIZE],
    }];

    assert_eq!(
        <super::BlockDeviceAdapter<MockReader> as embedded_sdmmc::BlockDevice>::write(
            &device,
            &blocks,
            embedded_sdmmc::BlockIdx(0),
        ),
        Err(super::StorageError::Unsupported)
    );
}
