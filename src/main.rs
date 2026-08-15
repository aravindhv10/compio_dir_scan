use std::{io::Write, os::fd::AsRawFd};

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
    fn from_slice(indata: &[u8], name: &str) -> anyhow::Result<Self> {
        let memfd = memfd::MemfdOptions::default().create(name)?;
        memfd.as_file().write_all(indata);
        let pid = rustix::process::getpid();
        let fd = memfd.as_raw_fd();
        let fdpath = format!("/proc/{}/fd/{}", pid, fd);
        println!("raw fd path = {}", fdpath);
        std::thread::sleep(std::time::Duration::from_secs(1000));
        println!("raw fd path = {}", fdpath);
        let ret = Self::default();
        return Ok(ret);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let buf = tokio::fs::read("./video.mp4").await?;
    let res = VideoReader::from_slice(buf.as_slice(), "video.mp4");
    Ok(())
}
