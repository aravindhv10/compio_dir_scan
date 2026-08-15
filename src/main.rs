struct VideoReader {
    data: Vec<u8>,
}

impl Default for VideoReader {
    fn default() -> Self {
        const SIZE_FRAME: usize = 1280 * 720 * 3;
        const FPS: usize = 8;
        const MIN_LENGTH: usize = 15;
        const MIN_TOTAL_SIZE: usize = SIZE_FRAME * FPS * MIN_LENGTH;
        Self {
            data: Vec::with_capacity(MIN_TOTAL_SIZE),
        }
    }
}

impl VideoReader {
    fn from_slice(indata: &[u8]) -> anyhow::Result<Self> {
        let ret = Self::default();

        return Ok(ret);
    }
}

fn main() -> anyhow::Result<()> {
    Ok(())
}
