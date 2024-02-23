use crate::utils::{to_cstring, ToU32Result};
use std::ptr::null_mut;

use ffmpeg4_ffi::{
    extra::defs::{averror, averror_eof, eagain},
    sys::{self, AVMediaType_AVMEDIA_TYPE_VIDEO, AVFormatContext},
};

pub struct VideoDecoder {
    fmt_ctx: *mut sys::AVFormatContext,
    pkt: *mut sys::AVPacket,
    codec_ctx: *mut sys::AVCodecContext,
    frame: *mut sys::AVFrame,
    video_stream_idx: u32,
}

pub trait FrameWriter {
    fn on_frame_decoded(&mut self, frame: *mut sys::AVFrame);
}


macro_rules! deref {
    // Match a single identifier
    ($base:ident) => { 
        $base 
    };

    // Match a sequence of field accesses
    ($a:expr, $($rest:ident),+) => { 
        deref_impl!($a, $($rest),*)
    };

    // Match a sequence of dereferences
    ($a:ident, $($rest:ident),+) => { 
        deref_impl!($a, $($rest),*)
    };
}

macro_rules! deref_impl {
    // Base case: single identifier
    ($base:ident) => { 
        $base 
    };

    // Base case: single identifier
    ($a:expr, $b: ident) => { 
        *(*$a).$b
    };

    // Base case: single identifier
    ($a:ident, $b: ident) => { 
        *(*$a).$b
    };

    // Recursive case: dereference
    ($a:expr, $b:ident, $($rest:ident),+) => { 
        deref_impl!((*$a).$b, $($rest),*)
    };

    // Recursive case: dereference
    ($a:ident, $b:ident, $($rest:ident),+) => { 
        deref_impl!((*$a).$b, $($rest),*)
    };
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
                frame,
                video_stream_idx: best_stream_idx,
            })
        }
    }

    pub fn format_video(&mut self) {
        unsafe {
            let fmt = self.fmt_ctx;
            let streams = deref!(self.fmt_ctx, iformat, name);
            sys::sws_getContext(fmt_ctx.streams, srcH, srcFormat, dstW, dstH, dstFormat, flags, srcFilter, dstFilter, param)
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
                        let ret = sys::avcodec_receive_frame(self.codec_ctx, self.frame);

                        if ret == averror(eagain()) || ret == averror(averror_eof()) {
                            break;
                        }
                        ret.to_u32_result("an error happened while receiving the frame")?;
                        println!("{}", (*self.frame).format);
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
            sys::av_frame_free(&mut self.frame);
        }
    }
}
