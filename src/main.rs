extern crate term_size;
mod cli;
mod decoder;
mod utils;
mod video;

use std::env;

use cli::CliFrameWriter;
use decoder::VideoDecoder;
use ffmpeg4_ffi::sys;

fn main() -> Result<(), String> {
    let mut decoder = VideoDecoder::new("/dev/video0")?;
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!(
            "Please provide two arguments : 
Either
    cli <cli_filters>
Or
    <path> <video_filters>

cli_filters : [rgb, greyscale]
video_filters : [edges]
path: output path for the video
"
        );
        return Err("invalid arguments".to_owned());
    }

    let mut writer = match (args[1].as_str(), args[2].as_str()) {
        ("cli", "greyscale") => CliFrameWriter::new(cli::CliFilter::Greyscale),
        ("cli", "rgb") => CliFrameWriter::new(cli::CliFilter::Rgb),
        (_, _) => return Err("invalid arguments".to_string()),
    };
    decoder.decode_frames(&mut writer)?;

    Ok(())
}
