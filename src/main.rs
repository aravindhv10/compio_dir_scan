use std::{io::Write, os::fd::AsRawFd};

struct VideoReader {
    data: memmapix::Mmap,
}

impl VideoReader {
    fn from_slice(indata: &[u8], name: &str) -> anyhow::Result<Self> {
        let opt = memfd::MemfdOptions::default();
        let pid = rustix::process::getpid();

        let memfd = opt.create(name)?;
        let fd = memfd.as_raw_fd();
        let fdpath = format!("/proc/{}/fd/{}", pid, fd);

        memfd.as_file().write_all(indata);
        drop(indata);

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

        Ok(Self { data: memmap })
    }

    fn from_path(fdpath: impl AsRef<std::path::Path>, name: &str) -> anyhow::Result<Self> {
        let opt = memfd::MemfdOptions::default();
        let pid = rustix::process::getpid();

        let out_memfd = memfd::MemfdOptions::default().create(name.to_string() + "out")?;
        let out_fd = out_memfd.as_raw_fd();
        let out_fdpath = format!("/proc/{}/fd/{}", pid, out_fd);

        let res = std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(fdpath.as_ref())
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

        Ok(Self { data: memmap })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let buf = tokio::fs::read("./video.mp4").await?;
    let res = VideoReader::from_slice(buf.as_slice(), "video.mp4")?;
    Ok(())
}
