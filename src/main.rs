use std::{ffi::CString, ptr::null_mut};

use ffmpeg4_ffi::sys::{self, AVMediaType_AVMEDIA_TYPE_VIDEO};
use ffmpeg4_ffi::sys::AVFormatContext;

trait ToU32Result {
    fn to_u32_result(self, err_str: &str) -> Result<u32, String>;
}

impl ToU32Result for i32 {
    fn to_u32_result(self, err_str: &str) -> Result<u32, String> {
        if self >= 0 {
            Ok(self as u32)
        } else {
            Err(err_str.to_string())
        }
    }
}

struct VideoDecoder {
    fmt_ctx: *mut sys::AVFormatContext,
    pkt: *mut sys::AVPacket,
    codec: *mut sys::AVCodec,
    codec_ctx: *mut sys::AVCodecContext,
}

impl VideoDecoder {
    fn new(filename: &str) -> Result<Self, String> {
        unsafe {
            let mut fmt_ctx = sys::avformat_alloc_context()
                .as_mut()
                .ok_or("Could not aquire format context")?;
            let c_filename = CString::new(filename).expect("").as_ptr();
            sys::avformat_open_input(
                &mut (fmt_ctx as *mut sys::AVFormatContext),
                c_filename,
                null_mut(),
                null_mut(),
            )
            .to_u32_result(format!("{} : {}", "could not open file", filename).as_str())?;

            let mut pkt = sys::av_packet_alloc()
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

            let mut codec = sys::avcodec_find_decoder(best_stream_idx as u32)
                .as_mut()
                .ok_or("Could not aquire a codec")?;

            let mut codec_ctx = sys::avcodec_alloc_context3(codec)
                .as_mut()
                .ok_or("Could not aquire a codec context")?;

            sys::avcodec_open2(codec_ctx, codec, null_mut())
                .to_u32_result("Could not open codec")?;

            Ok(VideoDecoder {
                fmt_ctx,
                pkt,
                codec,
                codec_ctx,
            })
        }
    }
}
fn main() -> Result<(), &'static str> {
    let input: *mut sys::AVFrame = unsafe { sys::av_frame_alloc() };
    let output: *mut sys::AVFrame = unsafe { sys::av_frame_alloc() };
    unsafe { loop {} }
    Ok(())
}
