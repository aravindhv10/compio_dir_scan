use anyhow::{Context, anyhow};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;

pub fn decode_video_to_rgb(
    video_bytes: Vec<u8>,
    size_x: u16,
    size_y: u16,
) -> anyhow::Result<Vec<u8>> {
    // 1. Initialize GStreamer
    gst::init()?;

    // 2. Define the pipeline description string.
    // We disable emit-signals since we are pulling frames synchronously.
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
        size_x, size_y
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
    let buffer = gst::Buffer::from_mut_slice(video_bytes);
    appsrc.push_buffer(buffer)?;
    appsrc.end_of_stream()?; // Signals EOF to the execution pipeline

    // 6. Allocate a standard, local vector for frame byte gathering
    let mut final_rgb_data = Vec::new();

    // 7. Start the pipeline execution
    pipeline.set_state(gst::State::Playing)?;

    // 8. Pull frames sequentially out of the appsink channel.
    // This loop terminates automatically when GStreamer propagates an internal End-of-Stream (EOS).
    while let Ok(sample) = appsink.pull_sample() {
        if let Some(sample_buffer) = sample.buffer() {
            // Map the internal C-buffer elements directly into Rust safe memory spaces
            let map = sample_buffer
                .map_readable()
                .map_err(|_| anyhow!("Failed to map GStreamer buffer memory"))?;

            final_rgb_data.extend_from_slice(map.as_slice());
        }
    }

    // 9. Inspect the Bus to ensure the pipeline terminated due to normal EOS
    // rather than an active parsing runtime error.
    let bus = pipeline.bus().context("Pipeline contains no status bus")?;
    if let Some(msg) = bus.pop_filtered(&[gst::MessageType::Error, gst::MessageType::Eos]) {
        use gst::MessageView;
        if let MessageView::Error(err) = msg.view() {
            // Safe pipeline teardown before bubbling up the failure
            let _ = pipeline.set_state(gst::State::Null);
            return Err(anyhow!(
                "GStreamer execution failure: {} ({:?})",
                err.error(),
                err.debug()
            ));
        }
    }

    // 10. Tear down the structural pipeline components gracefully
    pipeline.set_state(gst::State::Null)?;

    Ok(final_rgb_data)
}

fn main() {
    let res = std::fs::read("./video.mp4");
    match res {
        Ok(o) => {
            match decode_video_to_rgb(
                /*video_bytes: Vec<u8> =*/ o,
                /*size_x: u16 =*/ 1280 as u16,
                /*size_y: u16 =*/ 720 as u16,
            ) {
                Ok(q) => {
                    println!("{}", q.len());
                }
                Err(e) => {}
            };
        }
        Err(e) => {}
    };
}
