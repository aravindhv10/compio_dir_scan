use rustix::fd::AsRawFd;

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
    opt: memfd::MemfdOptions,
    pid: rustix::process::Pid,
}

struct VideoReaderPayload {
    data: memmapix::Mmap,
}

impl VideoReader {
    fn new() -> Self {
        Self {
            opt: memfd::MemfdOptions::default(),
            pid: rustix::process::getpid(),
        }
    }

    fn get_fd_path(&self, fd: i32) -> String {
        format!("/proc/{}/fd/{}", self.pid, fd)
    }

    fn tensor_from_slice(&self, data: &[u8], name: &str) -> anyhow::Result<VideoReaderPayload> {
        let memfd = self.opt.create(name)?;
        let fdpath = self.get_fd_path(memfd.as_raw_fd());

        let out_memfd = self.opt.create(name.to_string() + "out")?;
        let out_fdpath = self.get_fd_path(out_memfd.as_raw_fd());

        let mut res: usize = 0;
        while res < data.len() {
            res += rustix::io::write(memfd.as_file(), &data[res..])?;
        }

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
                    return Ok(VideoReaderPayload { data: memmap });
                }
            }
            None => {
                return Err(anyhow::format_err!("ffmpeg failed with unknown error code"));
            }
        };
    }

    fn tensor_from_path(
        &self,
        fdpath: impl AsRef<std::path::Path>,
        name: &str,
    ) -> anyhow::Result<VideoReaderPayload> {
        let out_memfd = self.opt.create(name)?;
        let out_fdpath = self.get_fd_path(out_memfd.as_raw_fd());

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
                    return Ok(VideoReaderPayload { data: memmap });
                }
            }
            None => {
                return Err(anyhow::format_err!("ffmpeg failed with unknown error code"));
            }
        };
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let buf = tokio::fs::read("./video.mp4").await?;
    let slave = VideoReader::new();
    let res = slave.tensor_from_slice(buf.as_slice(), "video.mp4")?;
    let num_frames = res.data.len() / (1280 * 720 * 3);
    println!("total number of frames = {}", num_frames);
    Ok(())
}
