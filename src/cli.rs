use ffmpeg4_ffi::sys;

use crate::decoder::FrameWriter;

pub enum CliFilter {
    Rgb,
    Greyscale,
}

pub struct CliFrameWriter {
    filter: CliFilter,
}

#[inline]
fn yuv_to_rgb(
    frame: sys::AVFrame,
    x_idx: i32,
    y_idx: i32,
    x_ratio: i32,
    y_ratio: i32,
) -> (u8, u8, u8) {
    unsafe {
        let (u_offset, v_offset): (i32, i32) = if (x_idx * x_ratio) & 1 == 0 {
            (1, 3)
        } else {
            (-1, 1)
        };
        let base_idx = (y_idx * y_ratio * frame.linesize[0] + 2 * x_idx * x_ratio) as isize;
        let y = *frame.data[0].offset(base_idx);
        let u = *frame.data[0].offset(base_idx + u_offset as isize);
        let v = *frame.data[0].offset(base_idx + v_offset as isize);

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
fn get_luminance(frame: sys::AVFrame, x: i32, y: i32, x_ratio: i32, y_ratio: i32) -> u8 {
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

fn greyscale(frame: sys::AVFrame, out: &mut String, x: i32, y: i32, x_ratio: i32, y_ratio: i32) {
    out.push(get_greyscale_representation(get_luminance(
        frame, x, y, x_ratio, y_ratio,
    )))
}

fn rgb(frame: sys::AVFrame, out: &mut String, x: i32, y: i32, x_ratio: i32, y_ratio: i32) {
    let (r, g, b) = yuv_to_rgb(frame, x, y, x_ratio, y_ratio);
    out.push_str(&rgb_color_string(" ", r, g, b));
}
impl FrameWriter for CliFrameWriter {
    fn on_frame_decoded(&mut self, frame: *mut ffmpeg4_ffi::sys::AVFrame) {
        unsafe {
            let frame = *frame;
            let mut str = String::new();
            let (w, h) = term_size::dimensions().expect("Could not acquire terminal dimensions");
            let (term_width, term_height) = (w as i32, h as i32);
            let x_ratio = frame.width / term_width;
            println!("{} {}", x_ratio, frame.width);
            let y_ratio = frame.height / term_height;

            for y in 0..term_height {
                for x in 0..term_width {
                    match self.filter {
                        CliFilter::Rgb => rgb(frame, &mut str, x, y, x_ratio, y_ratio),
                        CliFilter::Greyscale => greyscale(frame, &mut str, x, y, x_ratio, y_ratio),
                    }
                }
                str.push('\n');
            }
            print!("{}", str);
        }
    }
}

impl CliFrameWriter {
    pub fn new(filter: CliFilter) -> Self {
        CliFrameWriter { filter }
    }
}
