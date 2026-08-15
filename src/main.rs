use std::{io::Write, os::fd::AsRawFd};

fn ffmpeg_video_to_raw(
    path_in: impl AsRef<std::path::Path>,
    path_out: impl AsRef<std::path::Path>,
) -> anyhow::Result<std::process::ExitStatus> {
    let res = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(path_in.as_ref())
        .arg("-vf")
        .arg("fps=8,scale=1280:720")
        .arg("-nostdin")
        .arg("-loglevel")
        .arg("quiet")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgb24")
        .arg(path_out.as_ref())
        .status()?;
    Ok(res)
}

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

        let out_memfd = opt.create(name.to_string() + "out")?;
        let out_fd = out_memfd.as_raw_fd();
        let out_fdpath = format!("/proc/{}/fd/{}", pid, out_fd);

        let _ = memfd.as_file().write_all(indata)?;

        match ffmpeg_video_to_raw(
            /*path_in: impl AsRef<std::path::Path> =*/ fdpath,
            /*path_out: impl AsRef<std::path::Path> =*/ out_fdpath,
        )?
        .code()
        {
            Some(i) => {
                if i != 0 {
                    return Err(anyhow::format_err!("ffmpeg failed with error code {}", i));
                } else {
                    let memmap = unsafe { memmapix::Mmap::map(out_memfd.as_file()) }?;
                    return Ok(Self { data: memmap });
                }
            }
            None => {
                return Err(anyhow::format_err!("ffmpeg failed with unknown error code"));
            }
        };
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
