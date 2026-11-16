mod audio;
mod paste;
mod whistle;

use audio::AudioRecorder;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, State,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub language: String,
    pub auto_paste: bool,
    pub trigger_mode: String, // "toggle" | "hold"
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            auto_paste: true,
            trigger_mode: "toggle".to_string(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TranscriptionItem {
    pub id: String,
    pub text: String,
    pub latency_ms: u128,
    pub timestamp: String,
}

pub struct AppState {
    pub recorder: Mutex<AudioRecorder>,
    pub settings: Mutex<AppSettings>,
}

#[tauri::command]
fn get_whistle_status(app: AppHandle) -> Result<whistle::WhistleStatus, String> {
    whistle::get_status(&app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_whistle_assets(app: AppHandle) -> Result<whistle::WhistleStatus, String> {
    whistle::download_assets(&app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn update_settings(state: State<'_, AppState>, settings: AppSettings) {
    *state.settings.lock().unwrap() = settings;
}

#[tauri::command]
fn toggle_dictation(app: AppHandle) -> Result<(), String> {
    handle_dictation_toggle(&app);
    Ok(())
}

fn show_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        if let Ok(Some(monitor)) = overlay.primary_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let logical_w = (size.width as f64 / scale) as i32;
            let logical_h = (size.height as f64 / scale) as i32;
            let x = (logical_w - 320) / 2;
            let y = logical_h - 110;
            let _ = overlay.set_position(PhysicalPosition::new(
                (x as f64 * scale) as i32,
                (y as f64 * scale) as i32,
            ));
        }
        let _ = overlay.show();
    }
}

fn hide_overlay_delayed(app: AppHandle, delay_ms: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        if let Some(overlay) = app.get_webview_window("overlay") {
            let _ = overlay.hide();
        }
    });
}

fn start_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut rec = state.recorder.lock().unwrap();
    if rec.is_active() {
        return;
    }
    match rec.start(app.clone()) {
        Ok(()) => {
            show_overlay(app);
            let _ = app.emit(
                "dictation-state",
                serde_json::json!({ "phase": "recording" }),
            );
        }
        Err(e) => {
            let _ = app.emit(
                "dictation-state",
                serde_json::json!({ "phase": "error", "message": e.to_string() }),
            );
        }
    }
}

fn stop_and_transcribe(app: &AppHandle) {
    let state = app.state::<AppState>();
    let wav_path = std::env::temp_dir().join(format!("whistle_{}.wav", Uuid::new_v4()));
    {
        let mut rec = state.recorder.lock().unwrap();
        if !rec.is_active() {
            return;
        }
        if let Err(e) = rec.stop_to_wav(wav_path.clone()) {
            let _ = app.emit(
                "dictation-state",
                serde_json::json!({ "phase": "error", "message": e.to_string() }),
            );
            hide_overlay_delayed(app.clone(), 1200);
            return;
        }
    }

    let settings = state.settings.lock().unwrap().clone();
    let app_clone = app.clone();
    let _ = app.emit(
        "dictation-state",
        serde_json::json!({ "phase": "transcribing" }),
    );

    std::thread::spawn(move || {
        let t0 = Instant::now();
        let res = whistle::transcribe_wav(&app_clone, &wav_path, &settings.language);
        let _ = std::fs::remove_file(&wav_path);

        match res {
            Ok(text) if !text.is_empty() => {
                let latency_ms = t0.elapsed().as_millis();
                if settings.auto_paste {
                    if let Err(e) = paste::paste_into_active_field(&text) {
                        let _ = app_clone.emit(
                            "dictation-state",
                            serde_json::json!({ "phase": "error", "message": e.to_string() }),
                        );
                    } else {
                        let _ = app_clone.emit(
                            "dictation-state",
                            serde_json::json!({ "phase": "pasted" }),
                        );
                    }
                }
                let item = TranscriptionItem {
                    id: Uuid::new_v4().to_string(),
                    text,
                    latency_ms,
                    timestamp: Local::now().format("%H:%M:%S").to_string(),
                };
                let _ = app_clone.emit("transcription-complete", item);
                hide_overlay_delayed(app_clone, 650);
            }
            Ok(_) => {
                let _ = app_clone.emit(
                    "dictation-state",
                    serde_json::json!({ "phase": "idle", "message": "No speech detected" }),
                );
                hide_overlay_delayed(app_clone, 600);
            }
            Err(e) => {
                let _ = app_clone.emit(
                    "dictation-state",
                    serde_json::json!({ "phase": "error", "message": e.to_string() }),
                );
                hide_overlay_delayed(app_clone, 1800);
            }
        }
    });
}

fn handle_dictation_toggle(app: &AppHandle) {
    let active = {
        let state = app.state::<AppState>();
        let rec = state.recorder.lock().unwrap();
        rec.is_active()
    };
    if active {
        stop_and_transcribe(app);
    } else {
        start_recording(app);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            recorder: Mutex::new(AudioRecorder::new()),
            settings: Mutex::new(AppSettings::default()),
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let option_space = Shortcut::new(Some(Modifiers::ALT), Code::Space);
                    if shortcut == &option_space {
                        let mode = {
                            let state = app.state::<AppState>();
                            state.settings.lock().unwrap().trigger_mode.clone()
                        };
                        match (mode.as_str(), event.state()) {
                            ("hold", ShortcutState::Pressed) => start_recording(app),
                            ("hold", ShortcutState::Released) => stop_and_transcribe(app),
                            ("toggle", ShortcutState::Pressed) => handle_dictation_toggle(app),
                            _ => {}
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let option_space = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            app.global_shortcut().register(option_space)?;

            let show_item = MenuItem::with_id(app, "show", "Open WhistleFlow", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_whistle_status,
            download_whistle_assets,
            get_settings,
            update_settings,
            toggle_dictation
        ])
        .run(tauri::generate_context!())
        .expect("error while running WhistleFlow");
}
