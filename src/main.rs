use anyhow::{Context, anyhow};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use std::sync::Arc;
use std::sync::Mutex;

pub async fn decode_video_to_rgb(
    video_bytes: Vec<u8>,
    fps: f32,
    size_x: u16,
    size_y: u16,
) -> anyhow::Result<Vec<u8>> {
    // 1. Initialize GStreamer (safe to call multiple times)
    gst::init()?;

    // 2. Define the pipeline description string.
    // We use appsrc named 'mysrc' and appsink named 'mysink' to interact with them in code.
    let pipeline_str = format!(
        "appsrc name=mysrc block=true max-bytes=104857600 ! \
         typefind ! \
         decodebin ! \
         videorate ! \
         video/x-raw,fps={}/1 ! \
         videoconvert ! \
         videoscale ! \
         video/x-raw,format=RGB,width={},height={} ! \
         appsink name=mysink sync=false emit-signals=true",
        fps as i32, size_x, size_y
    );

    // 3. Parse the pipeline from the string descriptor
    let pipeline = gst::parse::launch(&pipeline_str)?
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| anyhow!("Failed to cast element to Pipeline"))?;

    // 4. Extract references to our source and sink components
    let appsrc = pipeline
        .by_name("mysrc")
        .context("Failed to find appsrc in pipeline")?
        .dynamic_cast::<gst_app::AppSrc>()
        .map_err(|_| anyhow!("Failed to cast to AppSrc"))?;

    let appsink = pipeline
        .by_name("mysink")
        .context("Failed to find appsink in pipeline")?
        .dynamic_cast::<gst_app::AppSink>()
        .map_err(|_| anyhow!("Failed to cast to AppSink"))?;

    // 5. Push the memory buffer into appsrc
    // We wrap the Vec into a GStreamer Buffer object
    let buffer = gst::Buffer::from_mut_slice(video_bytes);
    appsrc.push_buffer(buffer)?;
    appsrc.end_of_stream()?; // Inform the pipeline no more bytes are coming

    // 6. Setup thread-safe vector to accumulate raw pixel bytes asynchronously
    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let output_buffer_clone = Arc::clone(&output_buffer);

    // 7. Configure the appsink callback to intercept decoded frames
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let sample_buffer = sample.buffer().ok_or(gst::FlowError::Error)?;

                // Map the GStreamer internal buffer memory directly into memory space
                let map = sample_buffer
                    .map_readable()
                    .map_err(|_| gst::FlowError::Error)?;

                let mut lock = output_buffer_clone.lock().unwrap();
                lock.extend_from_slice(map.as_slice());

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    // 8. Start the pipeline execution
    pipeline.set_state(gst::State::Playing)?;

    // 9. Await processing completion via the GStreamer Bus
    let bus = pipeline.bus().context("Pipeline contains no bus")?;

    // Using spawn_blocking since bus monitoring blocks the executing thread
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Eos(_) => break, // Successful end of processing loop
                MessageView::Error(err) => {
                    return Err(anyhow!(
                        "GStreamer Error: {} ({:?})",
                        err.error(),
                        err.debug()
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    })
    .await??;

    // 10. Clean up pipeline state and return ownership of the linear RGB vector
    pipeline.set_state(gst::State::Null)?;

    let final_data = Arc::try_unwrap(output_buffer)
        .map_err(|_| anyhow!("Mutex binding references still held"))?
        .into_inner()?;

    Ok(final_data)
}

#[compio::main]
async fn main() -> std::io::Result<()> {
    Ok(())
}
