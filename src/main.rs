use anyhow::{Context, anyhow};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;

/// Guard struct to ensure GStreamer pipeline is shut down on drop (RAII)
struct PipelineGuard<'a>(&'a gst::Pipeline);

impl<'a> Drop for PipelineGuard<'a> {
    fn drop(&mut self) {
        let _ = self.0.set_state(gst::State::Null);
    }
}

struct VideoReader {
    pipeline_str: String,
}

impl VideoReader {
    pub fn new() -> Self {
        let pipeline_str = include_str!("pipeline_str.txt").to_string();

        Self {
            pipeline_str: pipeline_str,
        }
    }

    pub fn decode_video(&self, video_bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let pipeline = gst::parse::launch(&self.pipeline_str)?
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Failed to cast element to Pipeline"))?;

        let _guard = PipelineGuard(&pipeline);

        // 4. Extract handles
        let appsrc = pipeline
            .by_name("mysrc")
            .context("Failed to find appsrc")?
            .dynamic_cast::<gst_app::AppSrc>()
            .map_err(|_| anyhow!("Failed to cast to AppSrc"))?;

        let appsink = pipeline
            .by_name("mysink")
            .context("Failed to find appsink")?
            .dynamic_cast::<gst_app::AppSink>()
            .map_err(|_| anyhow!("Failed to cast to AppSink"))?;

        pipeline.set_state(gst::State::Playing)?;

        let buffer = gst::Buffer::from_slice(video_bytes);

        appsrc.push_buffer(buffer)?;
        appsrc.end_of_stream()?;

        const FRAME_SIZE: usize = 1280 * 720 * 3;
        const MIN_FRAMES: usize = 8 * 15;
        const MIN_CAPACITY: usize = FRAME_SIZE * MIN_FRAMES;
        let mut final_rgb_data = Vec::with_capacity(FRAME_SIZE * 8 * 15); // Pre-reserve 15 seconds @ 8FPS

        // 8. Pull frames from appsink
        while let Ok(sample) = appsink.pull_sample() {
            if let Some(sample_buffer) = sample.buffer() {
                let map = sample_buffer
                    .map_readable()
                    .map_err(|_| anyhow!("Failed to map GStreamer buffer memory"))?;

                final_rgb_data.extend_from_slice(map.as_slice());
            }
        }

        // 9. Check Bus for runtime errors
        if let Some(bus) = pipeline.bus() {
            if let Some(msg) = bus.pop_filtered(&[gst::MessageType::Error, gst::MessageType::Eos]) {
                if let gst::MessageView::Error(err) = msg.view() {
                    return Err(anyhow!(
                        "GStreamer pipeline error: {} ({:?})",
                        err.error(),
                        err.debug()
                    ));
                }
            }
        }

        Ok(final_rgb_data)
    }
}

pub fn decode_video_to_rgb(video_bytes: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    // 1. Initialize GStreamer
    gst::init()?;

    // 2. Define pipeline string
    let pipeline_str = format!(
        "appsrc name=mysrc block=true max-bytes=104857600 ! \
         typefind ! \
         decodebin ! \
         videorate ! \
         video/x-raw,framerate=8/1 ! \
         videoconvert ! \
         videoscale ! \
         video/x-raw,format=RGB,width={},height={} ! \
         appsink name=mysink sync=false emit-signals=false",
        width, height
    );

    // 3. Parse pipeline
    let pipeline = gst::parse::launch(&pipeline_str)?
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| anyhow!("Failed to cast element to Pipeline"))?;

    // RAII guard guarantees set_state(Null) on function exit or early return
    let _guard = PipelineGuard(&pipeline);

    // 4. Extract handles
    let appsrc = pipeline
        .by_name("mysrc")
        .context("Failed to find appsrc")?
        .dynamic_cast::<gst_app::AppSrc>()
        .map_err(|_| anyhow!("Failed to cast to AppSrc"))?;

    let appsink = pipeline
        .by_name("mysink")
        .context("Failed to find appsink")?
        .dynamic_cast::<gst_app::AppSink>()
        .map_err(|_| anyhow!("Failed to cast to AppSink"))?;

    // 5. Start the pipeline FIRST
    pipeline.set_state(gst::State::Playing)?;

    // 6. Push buffer AFTER pipeline is playing
    let buffer = gst::Buffer::from_slice(video_bytes.to_vec());
    appsrc.push_buffer(buffer)?;
    appsrc.end_of_stream()?;

    // 7. Calculate single frame byte size & pre-allocate output vector
    let frame_size = (width * height * 3) as usize; // RGB = 3 bytes/pixel
    let mut final_rgb_data = Vec::with_capacity(frame_size * 8 * 15); // Pre-reserve 15 seconds @ 8FPS

    // 8. Pull frames from appsink
    while let Ok(sample) = appsink.pull_sample() {
        if let Some(sample_buffer) = sample.buffer() {
            let map = sample_buffer
                .map_readable()
                .map_err(|_| anyhow!("Failed to map GStreamer buffer memory"))?;

            final_rgb_data.extend_from_slice(map.as_slice());
        }
    }

    // 9. Check Bus for runtime errors
    if let Some(bus) = pipeline.bus() {
        if let Some(msg) = bus.pop_filtered(&[gst::MessageType::Error, gst::MessageType::Eos]) {
            if let gst::MessageView::Error(err) = msg.view() {
                return Err(anyhow!(
                    "GStreamer pipeline error: {} ({:?})",
                    err.error(),
                    err.debug()
                ));
            }
        }
    }

    Ok(final_rgb_data)
}

fn main() -> anyhow::Result<()> {
    let video_bytes = std::fs::read("./video.mp4").context("Failed to read video file")?;

    let rgb_data = decode_video_to_rgb(&video_bytes, 1280, 720)?;

    let frame_bytes = (1280 * 720 * 3) as usize;
    let total_frames = rgb_data.len() / frame_bytes;

    println!(
        "Decoded {} total bytes across {} frames.",
        rgb_data.len(),
        total_frames
    );

    Ok(())
}
