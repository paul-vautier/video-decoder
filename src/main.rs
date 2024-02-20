use std::{
    ffi::{CStr, CString},
    ops::Index,
    ptr::null_mut,
    slice::from_raw_parts,
};

use ffmpeg4_ffi::{
    extra::defs::{averror, averror_eof, eagain},
    sys::{self, AVFrame, AVMediaType_AVMEDIA_TYPE_VIDEO},
};

extern crate term_size;

pub struct PixelData<'a>(&'a AVFrame);

pub trait Pixelable {
    fn pixels(&self) -> PixelData;
}

impl Pixelable for AVFrame {
    fn pixels(&self) -> PixelData {
        PixelData(self)
    }
}

impl<'a> Index<usize> for PixelData<'a> {
    type Output = [u8];
    fn index(&self, y: usize) -> &Self::Output {
        unsafe {
            from_raw_parts(
                self.0.data[0].wrapping_add(self.0.linesize[0] as usize * y),
                self.0.linesize[0] as usize,
            )
        }
    }
}

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
#[inline]
fn yuv_to_rgb(frame: AVFrame, x_idx: i32, y_idx: i32, x_ratio: i32, y_ratio: i32) -> (u8, u8, u8) {
    unsafe {
        let (u_offset, v_offset): (i32, i32) = if (x_idx * x_ratio) & 1 == 0 {
            (1, 3)
        } else {
            (-1, 1)
        };
        let (y_idx, x_idx) = ((y_idx * y_ratio), (2 * x_idx * x_ratio));
        let pixels = frame.pixels();

        let y = pixels[y_idx as usize][x_idx as usize];
        let u = pixels[y_idx as usize][(x_idx + u_offset) as usize];
        let v = pixels[y_idx as usize][(x_idx + v_offset) as usize];

        let y_float = y as f32;
        let u_float = u as f32 - 128.0;
        let v_float = v as f32 - 128.0;
        let r = (y_float + 1.402 * v_float).clamp(0.0, 255.0) as u8;
        let g = (y_float - 0.344136 * u_float - 0.714136 * v_float).clamp(0.0, 255.0) as u8;
        let b = (y_float + 1.772 * u_float).clamp(0.0, 255.0) as u8;

        (r, g, b)
    }
}

#[inline]
fn get_luminance(frame: AVFrame, x: i32, y: i32, x_ratio: i32, y_ratio: i32) -> u8 {
    unsafe { *frame.data[0].offset((y * y_ratio * frame.linesize[0] + 2 * x * x_ratio) as isize) }
}

fn get_greyscale_representation(luminance: u8) -> char {
    match luminance {
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
    }
}

fn rgb_color_string(text: &str, r: u8, g: u8, b: u8) -> String {
    format!("\x1B[48;2;{};{};{}m{}", r, g, b, text)
}

fn greyscale(frame: AVFrame, out: &mut String, x: i32, y: i32, x_ratio: i32, y_ratio: i32) {
    out.push(get_greyscale_representation(get_luminance(
        frame, x, y, x_ratio, y_ratio,
    )))
}

fn rgb(frame: AVFrame, out: &mut String, x: i32, y: i32, x_ratio: i32, y_ratio: i32) {
    let (r, g, b) = yuv_to_rgb(frame, x, y, x_ratio, y_ratio);
    out.push_str(&rgb_color_string(" ", r, g, b));
}

fn main() -> Result<(), String> {
    let mut decoder = VideoDecoder::new("/dev/video0")?;
    decoder.decode_frames(|frame| unsafe {
        let frame = *frame;
        let mut str = String::new();
        let (w, h) = term_size::dimensions().expect("Could not acquire terminal dimensions");
        let (term_width, term_height) = (w as i32, h as i32);
        let x_ratio = frame.width / term_width;
        let y_ratio = frame.height / term_height;

        for y in 0..term_height {
            for x in 0..term_width {
                rgb(frame, &mut str, x, y, x_ratio, y_ratio);
            }
            str.push('\n');
        }
        print!("{}", str);
    })?;

    Ok(())
}
