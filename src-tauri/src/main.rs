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

use scap::capturer::{CGPoint, CGRect, CGSize, Capturer, Options};

use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    encoder::ImageEncoder,
    frame::{Frame, ImageFormat},
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};

#[derive(Clone, Serialize)]
struct Payload {
    image: Vec<u8>,
    width: u32,
    height: u32,
}

struct CaptureInit {
    enabled: Arc<AtomicBool>,
    app: tauri::AppHandle,
}

// This struct will be used to handle the capture events.
struct Capture {
    enabled: Arc<AtomicBool>,
    app: tauri::AppHandle,
    encoder: ImageEncoder,
}

impl GraphicsCaptureApiHandler for Capture {
    // The type of flags used to get the values from the settings.
    type Flags = CaptureInit;

    // The type of error that can occur during capture, the error will be returned from `CaptureControl` and `start` functions.
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create the struct. The flags can be passed from settings.
    fn new(flags: Self::Flags) -> Result<Self, Self::Error> {
        let encoder = ImageEncoder::new(ImageFormat::Png, ColorFormat::Rgba8);

        Ok(Self {
            app: flags.app,
            enabled: flags.enabled,
            encoder,
        })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Note: The frame has other uses too for example you can save a single for to a file like this:
        // frame.save_as_image("frame.png", ImageFormat::Png)?;
        // Or get the raw data like this so you have full control:
        let w = frame.width();
        let h = frame.height();

        let mut frame_buffer = frame.buffer()?;
        let data = frame_buffer.as_raw_buffer();

        println!("width : {}, height : {}, buffer size {}", w, h, data.len());

        println!("Got an image !");

        if !self.enabled.load(Ordering::Relaxed) {
            capture_control.stop();
        }

        let img: RgbaImage =
            ImageBuffer::from_vec(w.try_into().unwrap(), h.try_into().unwrap(), data.to_vec())
                .unwrap();

        let resized_image: RgbaImage = resize(&img, 80, 45, FilterType::Nearest);

        self.app
            .emit(
                "image",
                Payload {
                    image: resized_image.to_vec(),
                    width: resized_image.width(),
                    height: resized_image.height(),
                },
            )
            .unwrap();

        println!(
            "Recieved BGRA frame of width {} and height {}",
            frame.width(),
            frame.height(),
        );

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) closes.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture Session Closed");

        Ok(())
    }
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
            let primary_monitor = Monitor::primary().expect("There is no primary monitor");

            let settings = Settings::new(
                // Item To Captue
                primary_monitor,
                // Capture Cursor Settings
                CursorCaptureSettings::WithoutCursor,
                // Draw Borders Settings
                DrawBorderSettings::WithoutBorder,
                // The desired color format for the captured frame.
                ColorFormat::Rgba8,
                // Additional flags for the capture settings that will be passed to user defined `new` function.
                CaptureInit { app, enabled },
            )
            .unwrap();

            // Starts the capture and takes control of the current thread.
            // The errors from handler trait will end up here
            Capture::start(settings).expect("Screen Capture Failed");
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
