extern crate term_size;
mod cli;
mod decoder;
mod utils;
mod video;

use std::env;

use cli::{CliFrameWriter, CliFilter};
use decoder::VideoDecoder;
use ffmpeg4_ffi::sys;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!(
            "Invalid arguments, please provide arguments with format <input_file> <cli_args|video_args> 

Arguments format : 

    cli_args: cli <cli_filters> -- arguments for writing the video output to the terminal 
    cli_filters : [rgb, greyscale]

    video_args: <output_file> <video_filters> -- arguments for generating a video output
    video_filters: [] -- available video filters
    output_file: output path for the video");
        return Err("invalid arguments".to_owned());
    }

    let mut decoder = VideoDecoder::new(&args[1])?;
    let mut writer = match (args[2].as_str(), args[3].as_str()) {
        ("cli", "greyscale") => CliFrameWriter::new(CliFilter::Greyscale),
        ("cli", "rgb") => CliFrameWriter::new(CliFilter::Rgb),
        (_, _) => return Err("invalid arguments".to_string()),
    };
    decoder.decode_frames(&mut writer)?;

    Ok(())
}
