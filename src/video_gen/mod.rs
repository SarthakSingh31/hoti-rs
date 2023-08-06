pub mod image_manager;
pub mod subtitle;
pub mod ui;

use std::time::Duration;

use glam::UVec2;
use gstreamer::{prelude::*, Caps, ClockTime, ElementFactory, Fraction, Pipeline};
use gstreamer_app::{AppSrc, AppSrcCallbacks};
use image::RgbaImage;

use self::ui::VideoUI;

pub struct Mp3(Vec<u8>);

impl Mp3 {
    pub fn new(data: Vec<u8>) -> Self {
        Mp3(data)
    }

    pub fn duration(&self) -> Duration {
        mp3_metadata::read_from_slice(&self.0).unwrap().duration
    }
}

pub struct VideoFrameIter {
    current_frame_idx: u32,
    size: UVec2,
    frame_rate: u32,
    total_frames: u32,
    pub ui: VideoUI,
    pub updaters: Vec<Box<dyn ui::UiUpdater>>,
}

impl VideoFrameIter {
    pub fn new(size: UVec2, frame_rate: u32, duration: Duration) -> Self {
        VideoFrameIter {
            current_frame_idx: 0,
            size,
            frame_rate,
            total_frames: (duration.as_secs_f64() * frame_rate as f64).round() as u32,
            ui: VideoUI::default(),
            updaters: Vec::default(),
        }
    }

    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    pub fn total_frames(&self) -> u32 {
        self.total_frames
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs((self.total_frames / self.frame_rate) as u64)
    }

    pub async fn encode_h264(mut self, audio_in: &str, video_out: &str) {
        // Initialize GStreamer
        gstreamer::init().unwrap();

        // Create the pipeline
        let pipeline = Pipeline::new(Some("image-sequence"));

        // Create the appsrc element
        let appsrc = ElementFactory::make("appsrc").build().unwrap();

        // Create the video convert element
        let video_convert = ElementFactory::make("videoconvert").build().unwrap();

        // Create the x264enc element
        let x264enc = ElementFactory::make("x264enc").build().unwrap();

        // Create the queue element
        let video_queue = ElementFactory::make("queue").build().unwrap();

        // Add and link the elements
        pipeline
            .add_many(&[&appsrc, &video_convert, &x264enc, &video_queue])
            .unwrap();
        gstreamer::Element::link_many(&[&appsrc, &video_convert, &x264enc, &video_queue]).unwrap();

        let appsrc = appsrc.downcast::<AppSrc>().unwrap();
        appsrc.set_format(gstreamer::Format::Time);
        appsrc.set_caps(Some(
            &Caps::builder("video/x-raw")
                .field("format", "RGBA")
                .field("width", self.size.x as i32)
                .field("height", self.size.y as i32)
                .field("framerate", Fraction::new(self.frame_rate as i32, 1))
                .build(),
        ));

        appsrc.set_callbacks(
            AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    match self.next() {
                        Some((idx, frame)) => {
                            // Wrap the data in a GStreamer buffer
                            let mut buffer = gstreamer::Buffer::from_mut_slice(frame.into_raw());

                            // Set the duration of the buffer
                            let duration = ClockTime::from_seconds(1) / self.frame_rate as u64;
                            let buffer_ref = buffer.get_mut().unwrap();
                            buffer_ref.set_duration(duration);
                            buffer_ref.set_pts(duration * idx as u64);

                            appsrc.push_buffer(buffer).unwrap();
                        }
                        None => {
                            appsrc.end_of_stream().unwrap();
                        }
                    }
                })
                .build(),
        );

        let audio_filesrc = ElementFactory::make("filesrc").build().unwrap();
        audio_filesrc.set_property("location", audio_in);

        // Create the decodebin element
        let audio_decodebin = ElementFactory::make("decodebin").build().unwrap();

        // Create the audio convert element
        let audio_convert = ElementFactory::make("audioconvert").build().unwrap();

        // Create the queue for audio
        let audio_queue = ElementFactory::make("queue").build().unwrap();

        pipeline
            .add_many(&[
                &audio_filesrc,
                &audio_decodebin,
                &audio_convert,
                &audio_queue,
            ])
            .unwrap();
        gstreamer::Element::link(&audio_filesrc, &audio_decodebin).unwrap();
        let audio_convert_weak = audio_convert.downgrade();
        audio_decodebin.connect_pad_added(move |_, src_pad| {
            let sink_pad = match audio_convert_weak.upgrade() {
                None => return,
                Some(s) => s.static_pad("sink").expect("cannot get sink pad from sink"),
            };

            src_pad
                .link(&sink_pad)
                .expect("Cannot link the decodebin source pad to the audioconvert sink pad");
        });
        gstreamer::Element::link(&audio_convert, &audio_queue).unwrap();

        // Create the h264parse element
        let h264parse = ElementFactory::make("h264parse").build().unwrap();

        let resample = ElementFactory::make("audioresample").build().unwrap();

        let avenc_aac = ElementFactory::make("avenc_aac").build().unwrap();

        // Create the mp4mux element
        let mp4mux = ElementFactory::make("mp4mux").build().unwrap();

        // Create the filesink element
        let filesink = ElementFactory::make("filesink").build().unwrap();
        filesink.set_property("location", video_out);

        pipeline
            .add_many(&[&h264parse, &resample, &avenc_aac, &mp4mux, &filesink])
            .unwrap();
        gstreamer::Element::link_many(&[&video_queue, &h264parse, &mp4mux, &filesink]).unwrap();

        // Link audio queue and muxer
        gstreamer::Element::link_many(&[&audio_queue, &resample, &avenc_aac, &mp4mux]).unwrap();

        // Start playing
        pipeline.set_state(gstreamer::State::Playing).unwrap();

        let mut eof_count = 0;
        // Wait until the pipeline finishes
        let bus = pipeline.bus().unwrap();
        for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;

            match msg.view() {
                MessageView::Eos(_) | MessageView::AsyncDone(_) => {
                    eof_count += 1;
                    if eof_count >= 2 {
                        break;
                    }
                }
                MessageView::Error(err) => {
                    println!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    break;
                }
                _ => {}
            }
        }

        pipeline.set_state(gstreamer::State::Null).unwrap();
    }
}

impl Iterator for VideoFrameIter {
    type Item = (u32, RgbaImage);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_frame_idx >= self.total_frames {
            None
        } else {
            for updater in &mut self.updaters {
                updater.update(self.current_frame_idx, &mut self.ui);
            }

            let mut frame = RgbaImage::new(self.size.x, self.size.y);

            if let Err(err) = self.ui.render(&mut frame) {
                panic!(
                    "Failed to render frame {} due to error: {}",
                    self.current_frame_idx, err
                );
            }

            self.current_frame_idx += 1;

            Some((self.current_frame_idx - 1, frame))
        }
    }
}
