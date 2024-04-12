// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::imageops::{resize, FilterType};
use image::{ImageBuffer, RgbImage, RgbaImage};
use serde::Serialize;
use std::io::ErrorKind::WouldBlock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{Manager, Window};

use scap::{
    capturer::{CGPoint, CGRect, CGSize, Capturer, Options},
    frame::Frame,
};

#[derive(Clone, Serialize)]
struct Payload {
    image: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct InnerCaptureState {
    pub thread_handle: Option<thread::JoinHandle<()>>,
    pub enabled: Arc<AtomicBool>,
}

impl InnerCaptureState {
    pub fn new() -> Self {
        Self {
            thread_handle: None,
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    // https://stackoverflow.com/questions/42043823/design-help-threading-within-a-struct
    pub fn start_capture(&mut self, app: tauri::AppHandle) {
        if self.thread_handle.is_some() {
            return;
        }

        self.enabled.store(true, Ordering::Relaxed);

        let app = app.clone();
        let enabled = self.enabled.clone();

        self.thread_handle = Some(thread::spawn(move || {
            // let one_second = Duration::new(1, 0);
            // let one_frame = one_second / 120;

            // let display = Display::primary().expect("Couldn't find primary display.");
            // let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
            // let (w, h) = (capturer.width(), capturer.height());

            //  while enabled.load(Ordering::Relaxed) {

            //     // Wait until there's a frame.

            //     let buffer: scrap::Frame<'_> = match capturer.frame() {
            //         Ok(buffer) => buffer,
            //         Err(error) => {
            //             if error.kind() == WouldBlock {
            //                 // Keep spinning.
            //                 thread::sleep(one_frame);
            //                 continue;
            //             } else {
            //                 panic!("Error: {}", error);
            //             }
            //         }
            //     };

            //     // Flip the ARGB image into a BGRA image.

            //     let mut bitflipped = Vec::with_capacity(w * h * 3);
            //     let stride = buffer.len() / h;

            //     for y in 0..h {
            //         for x in 0..w {
            //             let i = stride * y + 4 * x;
            //             bitflipped.extend_from_slice(&[buffer[i + 2], buffer[i + 1], buffer[i], 255]);
            //         }
            //     }

            //     let img: RgbaImage =
            //         ImageBuffer::from_vec(w.try_into().unwrap(), h.try_into().unwrap(), bitflipped)
            //             .unwrap();

            //     let resized_image: RgbaImage = resize(&img, 160, 90, FilterType::Nearest);
            //     // println!("Resized image !");

            //     app
            //         .emit(
            //             "image",
            //             Payload {
            //                 image: resized_image.to_vec(),
            //                 width: resized_image.width(),
            //                 height: resized_image.height(),
            //             },
            //         )
            //         .unwrap();
            // }
            // #1 Check if the platform is supported
            let supported = scap::is_supported();
            if !supported {
                println!("❌ Platform not supported");
                return;
            } else {
                println!("✅ Platform supported");
            }

            // #2 Check if we have permission to capture the screen
            let has_permission = scap::has_permission();
            if !has_permission {
                println!("❌ Permission not granted");
                return;
            } else {
                println!("✅ Permission granted");
            }

            // #3 Get recording targets (WIP)
            let targets = scap::get_targets();
            println!("🎯 Targets: {:?}", targets);

            // #4 Create Options
            let options = Options {
                fps: 30,
                targets,
                show_cursor: true,
                show_highlight: false,
                excluded_targets: None,
                output_type: scap::frame::FrameType::BGRAFrame,
                output_resolution: scap::capturer::Resolution::_480p,
                ..Default::default()
            };

            // #5 Create Recorder
            let mut recorder = Capturer::new(options);

            // #6 Start Capture
            recorder.start_capture();

            // #7 Capture 100 frames
            let mut start_time: u64 = 0;

            while enabled.load(Ordering::Relaxed) {
                let frame = recorder.get_next_frame().expect("Error");

                match frame {
                    Frame::YUVFrame(frame) => {
                        println!(
                            "Recieved YUV frame of width {} and height {} and pts {}",
                            frame.width, frame.height, frame.display_time
                        );
                    }
                    Frame::BGR0(frame) => {
                        println!(
                            "Received BGR0 frame of width {} and height {}",
                            frame.width, frame.height
                        );
                    }
                    Frame::RGB(frame) => {
                        if (start_time == 0) {
                            start_time = frame.display_time;
                        }
                        println!(
                            "Recieved RGB frame of width {} and height {} and time {}",
                            frame.width,
                            frame.height,
                            frame.display_time - start_time
                        );
                    }
                    Frame::RGBx(frame) => {
                        println!(
                            "Recieved RGBx frame of width {} and height {}",
                            frame.width, frame.height
                        );
                    }
                    Frame::XBGR(frame) => {
                        println!(
                            "Recieved xRGB frame of width {} and height {}",
                            frame.width, frame.height
                        );
                    }
                    Frame::BGRx(frame) => {
                        println!(
                            "Recieved BGRx frame of width {} and height {}",
                            frame.width, frame.height
                        );
                    }
                    Frame::BGRA(frame) => {
                        if (start_time == 0) {
                            start_time = frame.display_time;
                        }

                        let w = frame.width;
                        let h = frame.height;

                        let mut bitflipped = Vec::with_capacity((w * h * 3).try_into().unwrap());
                        let stride = frame.data.len() as i32 / h;

                        for y in 0..h {
                            for x in 0..w {
                                let i = stride * y + 4 * x;
                                bitflipped.extend_from_slice(&[
                                    frame.data[usize::try_from(i + 2).unwrap()],
                                    frame.data[usize::try_from(i + 1).unwrap()],
                                    frame.data[usize::try_from(i).unwrap()],
                                    255,
                                ]);
                            }
                        }

                        let img: RgbaImage = ImageBuffer::from_vec(
                            w.try_into().unwrap(),
                            h.try_into().unwrap(),
                            bitflipped,
                        )
                        .unwrap();

                        let resized_image: RgbaImage = resize(&img, 160, 90, FilterType::Nearest);

                        app.emit(
                            "image",
                            Payload {
                                image: resized_image.to_vec(),
                                width: resized_image.width(),
                                height: resized_image.height(),
                            },
                        )
                        .unwrap();

                        println!(
                            "Recieved BGRA frame of width {} and height {} and time {}",
                            frame.width,
                            frame.height,
                            frame.display_time - start_time
                        );
                    }
                }
            }

            // #8 Stop Capture
            recorder.stop_capture();
        }));
    }

    pub fn stop_capture(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            println!("Stopping capture !");
            self.enabled.store(false, Ordering::Relaxed);
            handle.join().unwrap();
        }
    }
}

pub struct CaptureState(pub Mutex<InnerCaptureState>);

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn start_capture(app: tauri::AppHandle, state: tauri::State<CaptureState>) {
    let mut state_guard = state.0.lock().unwrap();
    state_guard.start_capture(app);
}

#[tauri::command]
fn stop_capture(state: tauri::State<CaptureState>) {
    let mut state_guard = state.0.lock().unwrap();
    state_guard.stop_capture();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_shell::init())
        .manage(CaptureState(Mutex::new(InnerCaptureState::new())))
        .setup(move |app| Ok(()))
        .invoke_handler(tauri::generate_handler![start_capture, stop_capture])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
