use std::ptr::{null_mut};

use ffmpeg4_ffi::sys;
use crate::utils::to_cstring;
pub struct VideoFrameWriter {
    fmt_ctx: *mut sys::AVFormatContext,
    codec_ctx: *mut sys::AVCodecContext,
    stream: *mut sys::AVStream,
    pkt: *mut sys::AVPacket,
}

impl VideoFrameWriter {
    pub fn new(path: &str) -> Self{
        unsafe {

            let path = to_cstring(path);
            let mut fmt_ctx : *mut sys::AVFormatContext = null_mut();
            let format = sys::av_guess_format(to_cstring("v4l2").as_ptr(), null_mut(), null_mut());

            VideoFrameWriter {
                fmt_ctx,
                codec_ctx: null_mut(),
                stream: null_mut(), 
                pkt: null_mut(),  
            }
        }
    }
}
