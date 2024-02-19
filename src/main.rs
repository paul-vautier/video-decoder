use std::{
    ffi::{CStr, CString},
    ptr::null_mut,
};

use ffmpeg4_ffi::{sys::{self, AVMediaType_AVMEDIA_TYPE_VIDEO}, extra::defs::{averror, eagain, averror_eof}};

extern crate term_size;

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
    codec_ctx: *mut sys::AVCodecContext,
    frame: *mut sys::AVFrame,
    video_stream_idx: u32,
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
            let codec_params = (**(fmt_ctx.streams.wrapping_add(best_stream_idx as usize))).codecpar;
        
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

    fn decode_frames<F>(&mut self, on_frame_decoded: F) -> Result<(), String>
    where
        F: Fn(*mut sys::AVFrame),
    {
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
    decoder.decode_frames(|frame| unsafe {
        let frame = *frame;
        let mut str = String::new();
        let (w, h)  = term_size::dimensions().expect("Could not acquire terminal dimensions");
        let (w, h) = (w as i32, h as i32);
        let x_ratio = frame.width / w;
        let y_ratio = frame.height / h;
        for y in 0..h {
                for x in 0..w {
                    let luminance = frame.data[0].offset((y * y_ratio * frame.linesize[0] + 2 * x * x_ratio) as isize);
                    str.push(match *luminance {
                        0..=25 => ' ',
                        26..=50 => '.',
                        51..=75 => ':',
                        76..=100 => '-',
                        101..=125 => '=',
                        126..=150 => '+',
                        151..=175 => '*',
                        176..=200 => '#',
                        201..=225 => '%',
                        226..=255 => '@',
                    });
                }
                str.push('\n');
        }
        print!("{}", str);
    })?;

    Ok(())
}
