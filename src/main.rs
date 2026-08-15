use std::{io::Write, os::fd::AsRawFd};

use rustix::path::Arg;

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
        let opt = memfd::MemfdOptions::default();
        let pid = rustix::process::getpid();

        let memfd = opt.create(name)?;
        let fd = memfd.as_raw_fd();
        let fdpath = format!("/proc/{}/fd/{}", pid, fd);

        memfd.as_file().write_all(indata);

        let out_memfd = memfd::MemfdOptions::default().create(name.to_string() + "out")?;
        let out_fd = out_memfd.as_raw_fd();
        let out_fdpath = format!("/proc/{}/fd/{}", pid, out_fd);

        let res = std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(fdpath)
            .arg("-vf")
            .arg("fps=8,scale=1280:720")
            .arg("-nostdin")
            .arg("-loglevel")
            .arg("quiet")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgb24")
            .arg(&out_fdpath)
            .status()?;

        let memmap = unsafe { memmapix::Mmap::map(out_memfd.as_file()) }?;

        println!("Total length of raw file = {}", memmap.len());
        println!("Total fps of file = {}", memmap.len() / (1280 * 720 * 3));

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
