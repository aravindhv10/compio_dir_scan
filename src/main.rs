use std::io::Seek;

fn ffmpeg_video_to_raw_stdin_stdout(
    fd_in: std::process::Stdio,
    fd_out: std::process::Stdio,
) -> anyhow::Result<std::process::ExitStatus> {
    let res = std::process::Command::new("ffmpeg")
        // .arg("-init_hw_device")
        // .arg("vulkan=vk:0")
        // .arg("-hwaccel")
        // .arg("vulkan")
        .arg("-y")
        .arg("-i")
        .arg("/dev/fd/0")
        .arg("-vf")
        .arg("fps=8,scale=1280:720")
        .arg("-nostdin")
        .arg("-loglevel")
        .arg("quiet")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgb24")
        .arg("/dev/fd/1")
        .stdin(fd_in)
        .stdout(fd_out)
        .status()?;
    Ok(res)
}

fn ffmpeg_video_to_raw_file_stdout(
    path_in: impl AsRef<std::path::Path>,
    fd_out: std::process::Stdio,
) -> anyhow::Result<std::process::ExitStatus> {
    let res = std::process::Command::new("ffmpeg")
        // .arg("-init_hw_device")
        // .arg("vulkan=vk:0")
        // .arg("-hwaccel")
        // .arg("vulkan")
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
        .arg("/dev/fd/1")
        .stdout(fd_out)
        .status()?;
    Ok(res)
}

struct VideoReader {
    opt: memfd::MemfdOptions,
}

struct VideoReaderPayload {
    data: memmapix::Mmap,
}

impl VideoReader {
    fn new() -> Self {
        Self {
            opt: memfd::MemfdOptions::default(),
        }
    }

    fn tensor_from_slice(&self, data: &[u8], name: &str) -> anyhow::Result<VideoReaderPayload> {
        let memfd = self.opt.create(name)?;
        let out_memfd = self.opt.create(name.to_string() + "out")?;

        let mut res: usize = 0;
        while res < data.len() {
            res += rustix::io::write(memfd.as_file(), &data[res..])?;
        }

        memfd.as_file().seek(std::io::SeekFrom::Start(0))?;

        match ffmpeg_video_to_raw_stdin_stdout(
            std::process::Stdio::from(memfd.as_file().try_clone()?),
            std::process::Stdio::from(out_memfd.as_file().try_clone()?),
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

        match ffmpeg_video_to_raw_file_stdout(
            fdpath,
            std::process::Stdio::from(out_memfd.as_file().try_clone()?),
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
