use codec::raw::*;

use std::io::Write;
use std::{fs::File, io::Read, fs::OpenOptions};

fn main() {
    let mut input = File::open("C:/Users/quasi/Desktop/test.yuv").unwrap();
    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("C:/Users/quasi/Desktop/output.h264")
        .unwrap();

    let mut encoder = EncoderBuilder::new("h264_nvenc")
        .unwrap()
        .set_width(2560);
        .set_height(1440);
        .set_frame_rate(24);
        .set_format(PixelFormat::YUV420P);
        .build()
        .unwrap();

    let mut buf = vec![0u8; encoder.frame_size];
    let buf = buf.as_mut_slice();

    loop {
        if input.read_exact(buf).is_err() {
            loop {
                match encoder.read() {
                    Ok(Task::Wait) => {
                        break;
                    },
                    Ok(Task::Ready(chunk)) => {
                        output.write_all(chunk).unwrap();
                        encoder.flush();
                    },
                    _ => {
                        output.sync_all().unwrap();
                        break;
                    }
                }
            }

            return;
        }

        match encoder.write(buf).unwrap() {
            Task::Eof => {
                break;
            },
            Task::Ready(()) => {
                loop {
                    match encoder.read() {
                        Ok(Task::Wait) => {
                            break;
                        },
                        Ok(Task::Ready(chunk)) => {
                            output.write_all(chunk).unwrap();
                            encoder.flush();
                        },
                        _ => {
                            println!("read frame failed");
                            output.sync_all().unwrap();
                            break;
                        }
                    }
                }
            },
            Task::Wait => {
                loop {
                    match encoder.read() {
                        Ok(Task::Wait) => {
                            break;
                        },
                        Ok(Task::Ready(chunk)) => {
                            output.write_all(chunk).unwrap();
                            encoder.flush();
                        },
                        _ => {
                            output.sync_all().unwrap();
                            break;
                        }
                    }
                }
            }
        }
    }
}
