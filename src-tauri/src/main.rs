#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod ax_context;
mod config;
mod fetch;
mod insertion;
mod lang;
mod ocr;
mod panic_hook;
mod tray;
mod tts;
mod utils;
mod windows;
mod writing;

use config::get_config;
use debug_print::debug_println;
use insertion::{
    insert_translation_into_previous_input, remember_active_window, remember_active_window_command,
};
use parking_lot::Mutex;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sysinfo::{CpuExt, System, SystemExt};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;
use tray::{PinnedFromTrayEvent, PinnedFromWindowEvent};
use windows::{get_translator_window, CheckUpdateEvent, CheckUpdateResultEvent};

use crate::ax_context::{read_ax_context_narrow, read_ax_context_wide};
use crate::config::{clear_config_cache, get_config_content, ConfigUpdatedEvent};
use crate::fetch::fetch_stream;
use crate::lang::detect_lang;
use crate::ocr::{cut_image, finish_ocr, screenshot, start_ocr};
use crate::tts::synthesize_local_tts;
use crate::windows::{
    get_translator_window_always_on_top, get_writing_indicator_pending_lang,
    hide_inline_lookup_window, hide_quick_translator_window, hide_translator_window,
    hide_writing_indicator, recover_webview_visibility, show_action_manager_window,
    show_history_window, show_inline_lookup_window_command, show_quick_translator_window_command, show_translator_window_command,
    show_translator_window_with_selected_text_command, show_updater_window,
    show_writing_indicator, HISTORY_WIN_NAME, SETTINGS_WIN_NAME, TRANSLATOR_WIN_NAME,
};
use crate::writing::{finish_writing, write_to_input, writing_command};

use mouce::{Mouse, MouseActions};
use once_cell::sync::OnceCell;
#[cfg(debug_assertions)]
use specta_typescript::{formatter::prettier, Typescript};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tiny_http::{Response as HttpResponse, Server};
use tokio::runtime::{
    Builder as TokioRuntimeBuilder, EnterGuard as TokioEnterGuard, Runtime as TokioRuntime,
};

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
pub static ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);
pub static CPU_VENDOR: Mutex<String> = Mutex::new(String::new());
pub static SELECTED_TEXT: Mutex<String> = Mutex::new(String::new());
pub static PREVIOUS_PRESS_TIME: Mutex<u128> = Mutex::new(0);
pub static PREVIOUS_RELEASE_TIME: Mutex<u128> = Mutex::new(0);
pub static PREVIOUS_RELEASE_POSITION: Mutex<(i32, i32)> = Mutex::new((0, 0));
pub static RELEASE_THREAD_ID: Mutex<u32> = Mutex::new(0);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    version: String,
    current_version: String,
    body: Option<String>,
}

pub static UPDATE_RESULT: Mutex<Option<Option<UpdateResult>>> = Mutex::new(None);

fn set_update_result(app: &AppHandle, update_result: Option<UpdateResult>) {
    *UPDATE_RESULT.lock() = Some(update_result.clone());
    let _ = app.emit("update-status-changed", update_result);
}

fn init_tokio_runtime() -> &'static TokioRuntime {
    use std::sync::OnceLock;

    static RUNTIME: OnceLock<&'static TokioRuntime> = OnceLock::new();
    static ENTER_GUARD: OnceLock<&'static TokioEnterGuard<'static>> = OnceLock::new();
    static SET_HANDLE: OnceLock<()> = OnceLock::new();

    let runtime = RUNTIME.get_or_init(|| {
        Box::leak(Box::new(
            TokioRuntimeBuilder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to initialize Tokio runtime"),
        ))
    });

    ENTER_GUARD.get_or_init(|| Box::leak(Box::new(runtime.enter())));

    if SET_HANDLE.set(()).is_ok() {
        tauri::async_runtime::set(runtime.handle().clone());
    }

    runtime
}

#[tauri::command]
#[specta::specta]
fn get_update_result() -> (bool, Option<UpdateResult>) {
    if UPDATE_RESULT.lock().is_none() {
        return (false, None);
    }
    return (true, UPDATE_RESULT.lock().clone().flatten());
}
#[cfg(target_os = "macos")]
fn query_accessibility_permissions() -> bool {
    let trusted = macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    if trusted {
        print!("Application is totally trusted!");
    } else {
        print!("Application isn't trusted :(");
    }
    trusted
}

#[cfg(not(target_os = "macos"))]
fn query_accessibility_permissions() -> bool {
    return true;
}

#[inline]
fn launch_ipc_server(server: &Server) {
    for mut req in server.incoming_requests() {
        let mut selected_text = String::new();
        if req.as_reader().read_to_string(&mut selected_text).is_err() {
            let _ = req
                .respond(HttpResponse::from_string("bad request").with_status_code(400));
            continue;
        }
        let use_compact = config::get_config()
            .ok()
            .and_then(|c| c.use_compact_lookup)
            .unwrap_or(false);
        if use_compact {
            let window = windows::show_inline_lookup_window(false, true, false);
            utils::send_text(selected_text);
            let _ = window.set_focus();
        } else {
            utils::send_text(selected_text);
            remember_active_window();
            let window = windows::show_translator_window(false, true, false);
            let _ = window.set_focus();
            utils::show();
        }
        let response = HttpResponse::from_string("ok");
        let _ = req.respond(response);
    }
}

fn on_mouse_event(event: &mouce::common::MouseEvent) {
    match event {
        mouce::common::MouseEvent::Press(mouce::common::MouseButton::Left) => {
            let always_show_icons = config::get_config()
                .ok()
                .and_then(|c| c.always_show_icons)
                .unwrap_or(true);
            if !always_show_icons {
                return;
            }
            let current_press_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            *PREVIOUS_PRESS_TIME.lock() = current_press_time;
        }
        mouce::common::MouseEvent::Release(mouce::common::MouseButton::Left) => {
            let always_show_icons = config::get_config()
                .ok()
                .and_then(|c| c.always_show_icons)
                .unwrap_or(true);
            if !always_show_icons {
                windows::close_thumb();
                return;
            }
            let current_release_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let mut is_text_selected_event = false;
            let Ok((x, y)) = windows::get_mouse_location() else { return; };
            let (prev_release_x, prev_release_y) = { *PREVIOUS_RELEASE_POSITION.lock() };
            {
                *PREVIOUS_RELEASE_POSITION.lock() = (x, y);
            }
            let mouse_distance =
                (((x - prev_release_x).pow(2) + (y - prev_release_y).pow(2)) as f64).sqrt();
            let previous_press_time: u128;
            let previous_release_time: u128;
            {
                let previous_press_time_lock = PREVIOUS_PRESS_TIME.lock();
                let mut previous_release_time_lock = PREVIOUS_RELEASE_TIME.lock();
                previous_release_time = *previous_release_time_lock;
                *previous_release_time_lock = current_release_time;
                previous_press_time = *previous_press_time_lock;
            }
            let is_pressed = previous_release_time < previous_press_time;
            let pressed_time = current_release_time - previous_press_time;
            let is_double_click =
                current_release_time - previous_release_time < 700 && mouse_distance < 10.0;
            if is_pressed && pressed_time > 300 && mouse_distance > 20.0 {
                is_text_selected_event = true;
            }
            if previous_release_time != 0 && is_double_click {
                is_text_selected_event = true;
            }
            // Zero-IPC hit test against the cached thumb rect — the old
            // version queried outer_position/scale_factor/outer_size on
            // every left-button release system-wide.
            let is_click_on_thumb = windows::is_thumb_hit(x, y);
            // debug_println!("is_text_selected_event: {}", is_text_selected_event);
            // debug_println!("is_click_on_thumb: {}", is_click_on_thumb);
            if !is_text_selected_event && !is_click_on_thumb {
                windows::close_thumb();
                // println!("not text selected event");
                // println!("is_click_on_thumb: {}", is_click_on_thumb);
                // println!("mouse_distance: {}", mouse_distance);
                // println!("pressed_time: {}", pressed_time);
                // println!("released_time: {}", current_release_time - previous_release_time);
                // println!("is_double_click: {}", is_double_click);
                return;
            }

            if !is_click_on_thumb {
                if RELEASE_THREAD_ID.is_locked() {
                    return;
                }
                std::thread::spawn(move || {
                    // The automatic selection capture below synthesizes
                    // Ctrl+C. A console/terminal in the foreground reads
                    // that as SIGINT and aborts the running command
                    // (issue #548), so skip the capture — and the thumb
                    // — entirely there. The explicit hotkey summon path
                    // is unaffected.
                    #[cfg(target_os = "windows")]
                    {
                        if windows::foreground_window_is_console() {
                            debug_println!(
                                "Skip selection capture: foreground window is a console"
                            );
                            windows::close_thumb();
                            return;
                        }
                    }
                    #[cfg(target_os = "macos")]
                    {
                        if !utils::is_valid_selected_frame().unwrap_or(false) {
                            debug_println!("No valid selected frame");
                            windows::close_thumb();
                            return;
                        }
                    }

                let _lock = RELEASE_THREAD_ID.lock();
                let selected_text = windows::read_selected_text();
                if !selected_text.is_empty() {
                    {
                        *SELECTED_TEXT.lock() = selected_text;
                    }
                    // The selection icon is optional: with it off, users
                    // trigger translation purely via the global hotkey.
                    let show_selection_icon = config::get_config()
                        .ok()
                        .and_then(|c| c.show_selection_icon)
                        .unwrap_or(true);
                    if show_selection_icon {
                        windows::show_thumb(x, y);
                    }
                } else {
                    windows::close_thumb();
                }
                });
            } else {
                windows::close_thumb();
                let selected_text = (*SELECTED_TEXT.lock()).to_string();
                if !selected_text.is_empty() {
                    if RELEASE_THREAD_ID.is_locked() {
                        return;
                    }
                    // Summon off the hook callback: the full window
                    // pipeline used to run synchronously right here,
                    // stalling every mouse event behind it.
                    std::thread::spawn(move || {
                        let _lock = RELEASE_THREAD_ID.lock();
                        let use_compact = config::get_config()
                            .ok()
                            .and_then(|c| c.use_compact_lookup)
                            .unwrap_or(false);
                        if use_compact {
                            let window = windows::show_inline_lookup_window(false, true, false);
                            utils::send_text(selected_text);
                            let _ = window.set_focus();
                        } else {
                            remember_active_window();
                            let window = windows::show_translator_window(false, true, false);
                            utils::send_text(selected_text);
                            let _ = window.set_focus();
                        }
                    });
                }
            }
        }
        _ => {}
    }
}

fn bind_mouse_hook() {
    if !query_accessibility_permissions() {
        return;
    }

    // Mouse event hook requires `sudo` permission on linux.
    // Let's just skip it.
    if cfg!(target_os = "linux") {
        println!("mouse event hook skipped in linux!");
        return;
    }

    let mut mouse_manager = Mouse::new();

    let hook_result = mouse_manager.hook(Box::new(|event| {
        // This closure runs inside mouce's extern "system" hook procedure:
        // a panic cannot unwind across the FFI boundary and would abort
        // the whole process. catch_unwind is the last line of defense (the
        // panic hook has already written the panic to the log file) and
        // keeps the hook chain alive.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            on_mouse_event(event)
        }));
    }));

    match hook_result {
        Ok(_) => {
            println!("mouse event Hooked!");
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

fn main() {
    // Install first, before anything can panic: release builds have no
    // console, so the hook is the only way a panic leaves a trace.
    panic_hook::install();

    // Without a working WebView2 Runtime not a single window can be created,
    // so the app would only ever flash a frame and abort (discussion #1907).
    // Fail up front with an actionable dialog instead.
    #[cfg(target_os = "windows")]
    if let Err(err) = tauri::webview_version() {
        crate::windows::show_webview2_broken_dialog(&err.to_string());
        std::process::exit(1);
    }

    let _ = init_tokio_runtime();
    let silently = env::args().any(|arg| arg == "--silently");

    let mut sys = System::new();
    sys.refresh_cpu(); // Refreshing CPU information.
    if let Some(cpu) = sys.cpus().first() {
        let vendor_id = cpu.vendor_id().to_string();
        *CPU_VENDOR.lock() = vendor_id;
    }

    let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            get_config_content,
            get_update_result,
            clear_config_cache,
            show_translator_window_command,
            show_translator_window_with_selected_text_command,
            show_action_manager_window,
            show_history_window,
            show_updater_window,
            get_translator_window_always_on_top,
            fetch_stream,
            writing_command,
            write_to_input,
            finish_writing,
            show_writing_indicator,
            hide_writing_indicator,
            get_writing_indicator_pending_lang,
            insert_translation_into_previous_input,
            remember_active_window_command,
            detect_lang,
            screenshot,
            hide_translator_window,
            hide_inline_lookup_window,
            show_inline_lookup_window_command,
            show_quick_translator_window_command,
            hide_quick_translator_window,
            read_ax_context_narrow,
            read_ax_context_wide,
            start_ocr,
            finish_ocr,
            cut_image,
            synthesize_local_tts,
            recover_webview_visibility,
        ])
        .events(tauri_specta::collect_events![
            CheckUpdateEvent,
            CheckUpdateResultEvent,
            PinnedFromWindowEvent,
            PinnedFromTrayEvent,
            ConfigUpdatedEvent
        ]);

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            Typescript::default().formatter(prettier),
            "../src/tauri/bindings.ts",
        )
        .expect("Failed to export TypeScript bindings");

    let invoke_handler = specta_builder.invoke_handler();
    let specta_builder = Arc::new(specta_builder);

    let specta_builder_setup = specta_builder.clone();

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut app = tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);
            if let Err(e) = app
                .notification()
                .builder()
                .title("This app is already running!")
                .body("You can find it in the tray menu.")
                .show()
            {
                eprintln!("Error showing single-instance notification: {}", e);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--silently"]),
        ))
        .plugin(tauri_plugin_process::init())
        .setup(move |app| {
            specta_builder_setup.mount_events(app);
            let app_handle = app.handle();
            APP_HANDLE.get_or_init(|| app.handle().clone());
            tray::create_tray(&app_handle)?;
            app_handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            // Pre-create the thumb window only when the app was launched
            // explicitly. On autostart (`--silently`, typically right after
            // boot) every synchronous WebView2 creation in setup runs on the
            // main thread BEFORE the event loop starts: with disk and CPU
            // contended by every other autostart app, building the thumb,
            // dummy and translator webviews here stalled setup for tens of
            // seconds — a dead tray, dead hotkey and dead IPC while it
            // lasted. All of these windows have lazy-creation paths on first
            // use, so a silent start creates nothing and wakes instantly.
            if !silently {
                // create thumb window
                let _ = windows::get_thumb_window(0, 0);
                // If the selection icon can never show, suspend the pre-created
                // thumb's WebView2 renderer — a hidden webview whose controller
                // still reports IsVisible=TRUE composites forever (#1883).
                let config = config::get_config().unwrap_or_default();
                let always_show_icons = config.always_show_icons.unwrap_or(true);
                let show_selection_icon = config.show_selection_icon.unwrap_or(true);
                if !always_show_icons || !show_selection_icon {
                    if let Some(window) = app.get_webview_window(windows::THUMB_WIN_NAME) {
                        windows::set_webview_visibility(&window, false);
                    }
                }
            }
            // Pre-create the Quick Translator and writing-indicator panels —
            // but ONLY on macOS. There the raw cocoa msg_send calls (setLevel:,
            // etc.) must run on the AppKit main thread; if we let creation
            // happen lazily inside an async Tauri command (which runs on a
            // tokio worker thread), macOS aborts the process with
            // "Must only be used from the main thread" / EXC_BREAKPOINT.
            //
            // On Windows neither constraint applies, and pre-creating these
            // hidden transparent WebView2 windows kept their renderers
            // compositing at full frame rate from launch, burning CPU while
            // the app idled in the tray (#1883, #1886). They are created
            // lazily on first use instead.
            #[cfg(target_os = "macos")]
            {
                let _ = windows::get_quick_translator_window();
                let _ = windows::get_writing_indicator_window();
            }
            if silently {
                // Autostart: defer the translator window (and the thumb
                // above) to the first summon — creating them here is what
                // froze the app for a long while after boot. See the comment
                // at the thumb pre-creation above.
                debug_println!("silent start: windows deferred to first use");
            } else {
                let window = get_translator_window(false, false, false);
                if let Err(e) = window.set_focus() {
                    eprintln!("Error focusing translator window: {}", e);
                }
                if let Err(e) = window.show() {
                    eprintln!("Error showing translator window: {}", e);
                }
            }
            if !query_accessibility_permissions() {
                if let Some(window) = app.get_webview_window(TRANSLATOR_WIN_NAME) {
                    let _ = window.minimize();
                }
                if let Err(e) = app
                    .notification()
                    .builder()
                    .title("Accessibility permissions")
                    .body("Please grant accessibility permissions to the app")
                    .icon("icon.png")
                    .show()
                {
                    eprintln!("Error showing accessibility notification: {}", e);
                }
            }
            std::thread::spawn(move || {
                #[cfg(target_os = "windows")]
                {
                    let server = match Server::http("127.0.0.1:62007") {
                        Ok(server) => server,
                        Err(e) => {
                            eprintln!("Error starting IPC server on 127.0.0.1:62007: {}", e);
                            return;
                        }
                    };
                    launch_ipc_server(&server);
                }
                #[cfg(not(target_os = "windows"))]
                {
                    use std::path::Path;
                    let path = Path::new("/tmp/openai-translator.sock");
                    std::fs::remove_file(path).unwrap_or_default();
                    let server = match Server::http_unix(path) {
                        Ok(server) => server,
                        Err(e) => {
                            eprintln!("Error starting IPC unix server: {}", e);
                            return;
                        }
                    };
                    launch_ipc_server(&server);
                }
            });

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    // MUST be the async sleep: std::thread::sleep here parks a
                    // tokio runtime worker for 10 minutes, and with enough
                    // concurrently blocked workers every tauri command (all
                    // webview IPC) hangs - the app looks frozen while the UI
                    // process is perfectly healthy.
                    tokio::time::sleep(std::time::Duration::from_secs(60 * 10)).await;
                    let builder = handle.updater_builder();
                    let Ok(updater) = builder.build() else {
                        eprintln!("Error building updater in periodic check");
                        continue;
                    };

                    match updater.check().await {
                        Ok(Some(update)) => {
                            set_update_result(
                                &handle,
                                Some(UpdateResult {
                                    version: update.version,
                                    current_version: update.current_version,
                                    body: update.body,
                                }),
                            );
                            if let Err(e) = tray::create_tray(&handle) {
                                eprintln!("Error recreating tray: {}", e);
                            }
                        }
                        Ok(None) => {
                            let had_update = matches!(*UPDATE_RESULT.lock(), Some(Some(_)));
                            set_update_result(&handle, None);
                            if had_update {
                                if let Err(e) = tray::create_tray(&handle) {
                                    eprintln!("Error recreating tray: {}", e);
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            });
            let handle = app_handle.clone();
            PinnedFromWindowEvent::listen_any(app_handle, move |event| {
                let pinned = event.payload.pinned();
                ALWAYS_ON_TOP.store(*pinned, Ordering::Release);
                if let Err(e) = tray::create_tray(&handle) {
                    eprintln!("Error recreating tray: {}", e);
                }
            });

            let handle = app_handle.clone();
            ConfigUpdatedEvent::listen_any(app_handle, move |_event| {
                clear_config_cache();
                if let Err(e) = tray::create_tray(&handle) {
                    eprintln!("Error recreating tray: {}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(invoke_handler)
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    #[cfg(target_os = "macos")]
    {
        let config = config::get_config_by_app(app.handle()).unwrap_or_default();
        if config.hide_the_icon_in_the_dock.unwrap_or(true) {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        } else {
            app.set_activation_policy(tauri::ActivationPolicy::Regular);
        }
    }

    app.run(|app, event| match event {
        tauri::RunEvent::Ready => {
            bind_mouse_hook();
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let builder = handle.updater_builder();
                let updater = match builder.build() {
                    Ok(updater) => updater,
                    Err(e) => {
                        eprintln!("Error building updater in initial check: {}", e);
                        return;
                    }
                };

                match updater.check().await {
                    Ok(Some(update)) => {
                        set_update_result(
                            &handle,
                            Some(UpdateResult {
                                version: update.version,
                                current_version: update.current_version,
                                body: update.body,
                            }),
                        );
                        if let Err(e) = tray::create_tray(&handle) {
                            eprintln!("Error recreating tray: {}", e);
                        }
                        let config = get_config().unwrap_or_default();
                        if config.automatic_check_for_updates.is_none()
                            || config
                                .automatic_check_for_updates
                                .is_some_and(|x| x == true)
                        {
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            show_updater_window();
                        }
                    }
                    Ok(None) => {
                        let had_update = matches!(*UPDATE_RESULT.lock(), Some(Some(_)));
                        set_update_result(&handle, None);
                        if had_update {
                            if let Err(e) = tray::create_tray(&handle) {
                                eprintln!("Error recreating tray: {}", e);
                            }
                        }
                    }
                    Err(_) => {}
                }
            });
        }
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            if label == TRANSLATOR_WIN_NAME {
                windows::do_hide_translator_window();

                api.prevent_close();
                return;
            }

            // The utility windows stay alive, hidden: destroying a WebView2
            // controller hitches the main thread for a few hundred
            // milliseconds, which reads as a stutter on every close (the
            // whole app shares that thread). Hiding is instant, reopening
            // becomes instant too, and the hidden webview stops compositing
            // once its controller visibility is synced.
            if label == SETTINGS_WIN_NAME || label == HISTORY_WIN_NAME {
                if let Some(window) = app.get_webview_window(&label) {
                    windows::set_webview_visibility(&window, false);
                    let _ = window.hide();
                }
                api.prevent_close();
            }
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } => {
            if !has_visible_windows {
                remember_active_window();
                windows::show_translator_window(false, false, false);
            }
        }
        _ => {}
    });
}
