use crate::utils::{to_cstring, ToU32Result};

use std::ptr::null_mut;

use ffmpeg4_ffi::{
    extra::defs::{averror, averror_eof, eagain},
    sys::{self, sws_freeContext, AVFrame, AVMediaType_AVMEDIA_TYPE_VIDEO, SwsContext},
};

pub struct VideoDecoder {
    fmt_ctx: *mut sys::AVFormatContext,
    pkt: *mut sys::AVPacket,
    codec_ctx: *mut sys::AVCodecContext,
    input_frame: *mut sys::AVFrame,
    yuv_frame: Option<*mut sys::AVFrame>,
    sws_ctx: Option<*mut sys::SwsContext>,
    video_stream_idx: u32,
}

pub trait FrameWriter {
    fn on_frame_decoded(&mut self, frame: *mut sys::AVFrame);
}

fn create_sws_context(frame: sys::AVFrame) -> *mut sys::SwsContext {
    unsafe {
        sys::sws_getContext(
            frame.width,
            frame.height,
            frame.format,
            frame.width,
            frame.height,
            sys::AVPixelFormat_AV_PIX_FMT_YUYV422,
            sys::SWS_BILINEAR as i32,
            null_mut(),
            null_mut(),
            null_mut(),
        )
    }
}

fn create_frame(frame: *mut sys::AVFrame) -> *mut sys::AVFrame {
    unsafe {
        let frame = *frame;
        let to: *mut sys::AVFrame = sys::av_frame_alloc();
        (*to).width = frame.width;
        (*to).height = frame.height;
        (*to).format = sys::AVPixelFormat_AV_PIX_FMT_YUYV422;

        sys::av_frame_get_buffer(to, 0)
            .to_u32_result(" :( ")
            .unwrap();

        (*to).pts = 0;
        to
    }
}
impl VideoDecoder {
    /// Creates a new FFMPEG video decoder
    ///
    /// # Arguments
    /// * `filename` - The file path for the video input
    ///
    pub fn new(filename: &str) -> Result<Self, String> {
        unsafe {
            sys::avdevice_register_all();
            let frame: *mut sys::AVFrame = sys::av_frame_alloc();
            let fmt_ctx = sys::avformat_alloc_context()
                .as_mut()
                .ok_or("Could not aquire format context")?;

            sys::avformat_open_input(
                &mut (fmt_ctx as *mut sys::AVFormatContext),
                to_cstring(filename).as_ptr(),
                null_mut(),
                null_mut(),
            )
            .to_u32_result(format!("{} : {}", "could not open file", filename).as_str())?;

            let pkt = sys::av_packet_alloc()
                .as_mut()
                .ok_or("Could not aquire a pkt")?;

            let best_stream_idx = sys::av_find_best_stream(
                fmt_ctx,
                AVMediaType_AVMEDIA_TYPE_VIDEO,
                -1,
                -1,
                null_mut(),
                0,
            )
            .to_u32_result("could not find video stream")?;
            let codec_params =
                (**(fmt_ctx.streams.wrapping_add(best_stream_idx as usize))).codecpar;

            let codec = sys::avcodec_find_decoder((*codec_params).codec_id)
                .as_mut()
                .ok_or("Could not aquire a codec")?;
            let codec_ctx = sys::avcodec_alloc_context3(codec)
                .as_mut()
                .ok_or("Could not aquire a codec context")?;
            sys::avcodec_parameters_to_context(codec_ctx, codec_params);
            sys::avcodec_open2(codec_ctx, codec, null_mut())
                .to_u32_result("Could not open codec")?;

            Ok(VideoDecoder {
                fmt_ctx,
                pkt,
                codec_ctx,
                input_frame: frame,
                yuv_frame: None,
                sws_ctx: None,
                video_stream_idx: best_stream_idx,
            })
        }
    }

    pub fn format_video(&mut self) -> *mut AVFrame{
        unsafe {
            let frame = *self.input_frame;
            let sws_context = *self
                .sws_ctx
                .get_or_insert_with(|| create_sws_context(frame));
            let yuv_frame = *self
                .yuv_frame
                .get_or_insert_with(|| create_frame(self.input_frame));

            sys::sws_scale(
                sws_context,
                frame.data.as_ptr() as *const *const u8,
                frame.linesize.as_ptr(),
                0,
                frame.height,
                (*yuv_frame).data.as_ptr(),
                (*yuv_frame).linesize.as_ptr(),
            )
            .to_u32_result("could not change format")
            .unwrap();

            (*yuv_frame).pts = frame.pts;
            yuv_frame 
        }
    }

    /// Decode frames until none are left
    ///
    /// # Arguments
    /// * `decoder` - the delegate tasked with writing the frame content
    ///
    /// # Examples
    ///
    /// ```
    /// use cli::{CliFrameWriter, CliFilter}
    /// let mut writer = ...
    /// writer.decode_frames(&mut CliFrameWriter::new(CliFilter::Rgb));
    /// ```
    pub fn decode_frames(&mut self, decoder: &mut impl FrameWriter) -> Result<(), String> {
        unsafe {
            while sys::av_read_frame(self.fmt_ctx, self.pkt) >= 0 {
                if (*self.pkt).stream_index as u32 == self.video_stream_idx {
                    sys::avcodec_send_packet(self.codec_ctx, self.pkt)
                        .to_u32_result("Could not send packet to the AVCodec of the decoder")?;
                    loop {
                        let ret = sys::avcodec_receive_frame(self.codec_ctx, self.input_frame);

                        if ret == averror(eagain()) || ret == averror(averror_eof()) {
                            break;
                        }
                        ret.to_u32_result("an error happened while receiving the frame")?;
                        decoder.on_frame_decoded(self.format_video());
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        unsafe {
            sys::av_frame_free(&mut self.input_frame);
            let _ = self.sws_ctx.map(|ctx| sys::sws_freeContext(ctx));
            let _ = self
                .yuv_frame
                .map(|mut frame| sys::av_frame_free(&mut frame));
        }
    }
}
