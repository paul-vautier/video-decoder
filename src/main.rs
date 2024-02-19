use std::{
    ffi::{c_void, CStr, CString},
    ptr::{null, null_mut},
};

use ffmpeg4_ffi::{
    extra::defs::{averror, averror_eof, eagain},
    sys::{self, AVHWFramesContext, AVMediaType_AVMEDIA_TYPE_VIDEO},
};

trait ToU32Result {
    fn to_u32_result(self, err_str: &str) -> Result<u32, String>;
}

impl ToU32Result for i32 {
    fn to_u32_result(self, err_str: &str) -> Result<u32, String> {
        if self >= 0 {
            Ok(self as u32)
        } else {
            let mut description: [std::os::raw::c_char; sys::AV_ERROR_MAX_STRING_SIZE as usize] =
                [0; sys::AV_ERROR_MAX_STRING_SIZE as usize];
            let res: &CStr = unsafe {
                sys::av_strerror(
                    self,
                    description.as_mut_ptr(),
                    sys::AV_ERROR_MAX_STRING_SIZE as usize,
                );
                CStr::from_ptr(description.as_ptr())
            };
            Err(format!("{}. {}", res.to_string_lossy(), err_str))
        }
    }
}

struct VideoDecoder {
    fmt_ctx: *mut sys::AVFormatContext,
    pkt: *mut sys::AVPacket,
    codec: *mut sys::AVCodec,
    codec_ctx: *mut sys::AVCodecContext,
    frame: *mut sys::AVFrame,
    video_stream_idx: u32,
}

enum VideoResult {
    Finished,
    Decoded,
}

fn to_cstring(str: &str) -> CString {
    CString::new(str).expect("could not create cstring")
}
impl VideoDecoder {
    fn new(filename: &str) -> Result<Self, String> {
        unsafe {
        sys::avdevice_register_all();
            let frame: *mut sys::AVFrame = sys::av_frame_alloc();
            let fmt_ctx = sys::avformat_alloc_context()
                .as_mut()
                .ok_or("Could not aquire format context")?;

            let mut options: *mut sys::AVDictionary = null_mut();
            fmt_ctx.iformat = sys::av_find_input_format(to_cstring("v4l2").as_ptr());
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
            println!("{best_stream_idx} {}", best_stream_idx as isize);
            let codec_params = (**(fmt_ctx.streams.wrapping_add(best_stream_idx as usize))).codecpar;
        
            println!("dasda");
            let codec = sys::avcodec_find_decoder((*codec_params).codec_id)
                .as_mut()
                .ok_or("Could not aquire a codec")?;
            println!("aaa");
            let codec_ctx = sys::avcodec_alloc_context3(codec)
                .as_mut()
                .ok_or("Could not aquire a codec context")?;
            codec_ctx.pix_fmt = sys::AVPixelFormat_AV_PIX_FMT_RGB24;

            sys::avcodec_parameters_to_context(codec_ctx, codec_params);
            sys::avcodec_open2(codec_ctx, codec, null_mut())
                .to_u32_result("Could not open codec")?;
            Ok(VideoDecoder {
                fmt_ctx,
                pkt,
                codec,
                codec_ctx,
                frame,
                video_stream_idx: best_stream_idx,
            })
        }
    }

    fn decode_frames<F>(&mut self, output_file: &str, on_frame_decoded: F) -> Result<(), String>
    where
        F: Fn(*mut sys::AVFrame),
    {
        unsafe {
            while sys::av_read_frame(self.fmt_ctx, self.pkt) >= 0 {
                if (*self.pkt).stream_index as u32 == self.video_stream_idx {
                    sys::avcodec_send_packet(self.codec_ctx, self.pkt)
                        .to_u32_result("Could not send packet to the AVCodec of the decoder")?;
                    loop {
                        let ret = sys::avcodec_receive_frame(self.codec_ctx, self.frame)
                            .to_u32_result("error while receiving a frame from the decoder")?;
                        on_frame_decoded(self.frame);
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
fn main() -> Result<(), String> {
    let mut decoder = VideoDecoder::new("/dev/video0")?;
    println!("o");
    decoder.decode_frames("", |frame| unsafe {
        println!("frame format {}", (*frame).format);
    })?;

    Ok(())
}
