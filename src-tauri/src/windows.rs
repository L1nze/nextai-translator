use crate::config;
use crate::insertion::remember_active_window;
use crate::utils;
use crate::UpdateResult;
use crate::ALWAYS_ON_TOP;
use crate::APP_HANDLE;
#[cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
use debug_print::debug_println;
use enigo::*;
#[cfg(not(target_os = "windows"))]
use get_selected_text::get_selected_text;
use mouse_position::mouse_position::Mouse;
use serde_json::json;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{Emitter, Listener, LogicalPosition, Manager, PhysicalPosition};
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;

pub const TRANSLATOR_WIN_NAME: &str = "translator";
pub const SETTINGS_WIN_NAME: &str = "settings";
pub const ACTION_MANAGER_WIN_NAME: &str = "action_manager";
pub const UPDATER_WIN_NAME: &str = "updater";
pub const THUMB_WIN_NAME: &str = "thumb";
pub const HISTORY_WIN_NAME: &str = "history";
pub const INLINE_LOOKUP_WIN_NAME: &str = "inline_lookup";
pub const QUICK_TRANSLATOR_WIN_NAME: &str = "quick_translator";
pub const WRITING_INDICATOR_WIN_NAME: &str = "writing_indicator";
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub const SCREENSHOT_WIN_NAME: &str = "screenshot";

fn get_dummy_window() -> tauri::WebviewWindow {
    let app_handle = APP_HANDLE.get().unwrap();
    match app_handle.get_webview_window("dummy") {
        Some(window) => {
            debug_println!("Dummy window found!");
            window
        }
        None => {
            debug_println!("Create dummy window!");
            checked_build(
                tauri::WebviewWindowBuilder::new(
                    app_handle,
                    "dummy",
                    tauri::WebviewUrl::App("src/tauri/dummy.html".into()),
                )
                .title("Dummy")
                .visible(false),
            )
        }
    }
}

pub fn get_current_monitor() -> Option<tauri::Monitor> {
    let window = get_dummy_window();
    let (mouse_logical_x, mouse_logical_y): (i32, i32) = get_mouse_location().ok()?;
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let mut mouse_physical_position = PhysicalPosition::new(mouse_logical_x, mouse_logical_y);
    if cfg!(target_os = "macos") {
        mouse_physical_position =
            LogicalPosition::new(mouse_logical_x as f64, mouse_logical_y as f64)
                .to_physical(scale_factor);
    }
    window
        .available_monitors()
        .ok()
        .and_then(|monitors| {
            monitors
                .iter()
                .find(|monitor| {
                    let monitor_physical_size = monitor.size();
                    let monitor_physical_position = monitor.position();
                    mouse_physical_position.x >= monitor_physical_position.x
                        && mouse_physical_position.x
                            <= monitor_physical_position.x + (monitor_physical_size.width as i32)
                        && mouse_physical_position.y >= monitor_physical_position.y
                        && mouse_physical_position.y
                            <= monitor_physical_position.y + (monitor_physical_size.height as i32)
                })
                .cloned()
        })
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten())
}

pub fn get_mouse_location() -> Result<(i32, i32), String> {
    let position = Mouse::get_mouse_position();
    match position {
        Mouse::Position { x, y } => Ok((x, y)),
        Mouse::Error => Err("Error getting mouse position".to_string()),
    }
}

pub fn set_translator_window_always_on_top() -> bool {
    let handle = APP_HANDLE.get().unwrap();
    if let Some(window) = handle.get_webview_window(TRANSLATOR_WIN_NAME) {
        let always_on_top = ALWAYS_ON_TOP.load(Ordering::Acquire);

        if !always_on_top {
            let _ = window.set_always_on_top(true);
            ALWAYS_ON_TOP.store(true, Ordering::Release);
        } else {
            let _ = window.set_always_on_top(false);
            ALWAYS_ON_TOP.store(false, Ordering::Release);
        }
        ALWAYS_ON_TOP.load(Ordering::Acquire)
    } else {
        false
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_translator_window_always_on_top() -> bool {
    ALWAYS_ON_TOP.load(Ordering::Acquire)
}

#[cfg(target_os = "windows")]
enum SavedClipboard<'a> {
    Text(String),
    Image(arboard::ImageData<'a>),
    Empty,
}

#[cfg(target_os = "windows")]
fn save_clipboard() -> SavedClipboard<'static> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.get_text() {
            Ok(text) => SavedClipboard::Text(text),
            Err(_) => match clipboard.get_image() {
                Ok(image) => SavedClipboard::Image(image),
                Err(_) => SavedClipboard::Empty,
            },
        },
        Err(e) => {
            eprintln!("Error opening clipboard: {}", e);
            SavedClipboard::Empty
        }
    }
}

#[cfg(target_os = "windows")]
fn restore_clipboard(saved: SavedClipboard) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        match saved {
            SavedClipboard::Text(text) => {
                let _ = clipboard.set_text(text);
            }
            SavedClipboard::Image(image) => {
                let _ = clipboard.set_image(image);
            }
            SavedClipboard::Empty => {
                let _ = clipboard.clear();
            }
        }
    }
}

/// Window classes of console/terminal hosts on Windows. Selection capture
/// synthesizes Ctrl+C, and these windows interpret that keystroke as SIGINT
/// and abort the running command (issue #548), so the automatic mouse-hook
/// capture must never fire while one of them is in the foreground.
#[cfg(target_os = "windows")]
const CONSOLE_WINDOW_CLASSES: [&str; 5] = [
    "ConsoleWindowClass",            // conhost: cmd, powershell, any console application
    "CASCADIA_HOSTING_WINDOW_CLASS", // Windows Terminal
    "mintty",                        // Git Bash / MSYS2
    "PuTTY",                         // PuTTY
    "VirtualConsoleClass",           // ConEmu
];

/// True while the foreground window belongs to a console/terminal host.
#[cfg(target_os = "windows")]
pub fn foreground_window_is_console() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }
        let mut class_name = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class_name) as usize;
        if len == 0 {
            return false;
        }
        let class = String::from_utf16_lossy(&class_name[..len]);
        CONSOLE_WINDOW_CLASSES.contains(&class.as_str())
    }
}

/// Capture the current selection by synthesizing Ctrl+C and reading the
/// clipboard. The `get-selected-text` crate (and its vendored copy in
/// utils.rs) guesses readiness with fixed sleeps (50ms before, 100ms
/// after): when the focused app needs a moment longer to publish the
/// selection, the read returns the placeholder, the empty result is
/// dropped instead of sent, and the popup reopens still showing the
/// previous translation. Instead, poll the clipboard sequence number
/// until it actually changes, bounded at ~600ms.
///
/// `pub(crate)`: the mouse hook's selection branch uses this same polling
/// implementation instead of the old fixed-sleep crate path, so there is
/// exactly one selection-capture implementation.
#[cfg(target_os = "windows")]
pub(crate) fn read_selected_text() -> String {
    use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

    let saved = save_clipboard();
    let sequence_before = unsafe { GetClipboardSequenceNumber() };

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(enigo) => enigo,
        Err(e) => {
            eprintln!("Error creating enigo: {}", e);
            return String::new();
        }
    };
    // Release any stuck modifiers before synthesizing the copy.
    let _ = enigo.key(Key::Control, Direction::Release);
    let _ = enigo.key(Key::Alt, Direction::Release);
    let _ = enigo.key(Key::Shift, Direction::Release);
    let _ = enigo.key(Key::Control, Direction::Press);
    let _ = enigo.key(Key::C, Direction::Click);
    let _ = enigo.key(Key::Control, Direction::Release);

    let mut changed = false;
    for _ in 0..120 {
        if unsafe { GetClipboardSequenceNumber() } != sequence_before {
            changed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if !changed {
        restore_clipboard(saved);
        return String::new();
    }
    // Give apps that publish several formats (plain text, HTML, ...) a
    // moment to finish before reading.
    std::thread::sleep(Duration::from_millis(30));
    let text = arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .unwrap_or_default();
    restore_clipboard(saved);
    text
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn read_selected_text() -> String {
    match get_selected_text() {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Error getting selected text natively: {}", e);
            String::new()
        }
    }
}

// Reentrancy + auto-repeat guard for the summon pipeline below. The pipeline
// saves the clipboard, synthesizes Ctrl+C and polls it back, so overlapping
// runs (holding the hotkey down re-fires it at the keyboard repeat rate)
// would race the clipboard and fight over focus. Drop invocations that
// arrive while one is in flight or within 700ms of one starting.
static SUMMON_IN_FLIGHT: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
static LAST_SUMMON: parking_lot::Mutex<Option<std::time::Instant>> =
    parking_lot::Mutex::new(None);

#[tauri::command]
#[specta::specta]
pub async fn show_translator_window_with_selected_text_command() {
    {
        let mut last = LAST_SUMMON.lock();
        let now = std::time::Instant::now();
        if let Some(t) = *last {
            if now.duration_since(t) < Duration::from_millis(700) {
                return;
            }
        }
        *last = Some(now);
    }
    let Some(_summon_guard) = SUMMON_IN_FLIGHT.try_lock() else {
        return;
    };
    #[cfg(target_os = "windows")]
    unsafe {
        // The hotkey press granted this process the right to take the
        // foreground, but the selection capture below spends up to ~600ms
        // injecting Ctrl+C into the previous app, which can forfeit that
        // right before the focus grab at the end of the pipeline. Re-grant
        // it now (to "any process", i.e. ourselves later) while it is fresh.
        use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    remember_active_window();
    let config = config::get_config().ok();
    let restore_previous_position = config
        .as_ref()
        .and_then(|conf| conf.restore_previous_position)
        .unwrap_or(false);

    // Capture before any Windows window-state changes: showing or restoring
    // the popup can activate it and redirect Ctrl+C away from the source app.
    #[cfg(target_os = "windows")]
    let selected_text = read_selected_text();

    // Restore reused windows only after show() synchronizes tao's visibility
    // state; restoring a hidden minimized window first can flash it.
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(TRANSLATOR_WIN_NAME) {
        Some(window) => {
            #[cfg(not(target_os = "windows"))]
            let _ = window.unminimize();
            window
        }
        None => get_translator_window(false, false, false),
    };
    if !restore_previous_position {
        let target = compute_translator_position_near_cursor(&window);
        show_selection_window(&window, Some(target));
    } else {
        show_selection_window(&window, None);
    }

    #[cfg(not(target_os = "windows"))]
    let selected_text = read_selected_text();

    if !selected_text.trim().is_empty() {
        utils::send_text(selected_text);
    }

    // macOS initially reveals the window without activation; after capture,
    // synchronize its visibility before restoring and focusing it.
    #[cfg(target_os = "macos")]
    let _ = window.show();

    focus_translator_window(&window);
    utils::show();
}

/// Compute the cursor position clamped to its monitor before revealing the
/// window, so it never flashes at its previous position.
fn compute_translator_position_near_cursor(window: &tauri::WebviewWindow) -> PhysicalPosition<i32> {
    let current_monitor = get_current_monitor();
    let window_physical_size = window.outer_size().unwrap_or_default();
    let (mouse_logical_x, mouse_logical_y) = get_mouse_location().unwrap_or((0, 0));
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let mut mouse_physical_position = PhysicalPosition::new(mouse_logical_x, mouse_logical_y);
    if cfg!(target_os = "macos") {
        mouse_physical_position =
            LogicalPosition::new(mouse_logical_x as f64, mouse_logical_y as f64).to_physical(scale_factor);
    }

    let mut target = mouse_physical_position;
    if let Some(current_monitor) = current_monitor {
        let monitor_physical_size = current_monitor.size();
        let monitor_physical_position = current_monitor.position();

        if target.x < monitor_physical_position.x {
            target.x = monitor_physical_position.x;
        }
        if target.y < monitor_physical_position.y {
            target.y = monitor_physical_position.y;
        }
        if target.x + (window_physical_size.width as i32)
            > monitor_physical_position.x + (monitor_physical_size.width as i32)
        {
            target.x = monitor_physical_position.x
                + (monitor_physical_size.width as i32)
                - (window_physical_size.width as i32);
        }
        if target.y + (window_physical_size.height as i32)
            > monitor_physical_position.y + (monitor_physical_size.height as i32)
        {
            target.y = monitor_physical_position.y
                + (monitor_physical_size.height as i32)
                - (window_physical_size.height as i32);
        }
    }
    target
}

/// Position before revealing the window. Windows captures the selection first
/// and uses Tauri's show() so delayed pin changes cannot re-hide the popup via
/// stale visibility flags. macOS keeps the source app focused during capture.
fn show_selection_window(window: &tauri::WebviewWindow, position: Option<PhysicalPosition<i32>>) {
    if let Some(target) = position {
        let _ = window.set_position(target);
    }
    #[cfg(target_os = "macos")]
    {
        // orderFront: displays without making the window key, keeping the
        // keyboard with the app whose selection is about to be captured.
        let _ = window.with_webview(|webview| unsafe {
            use cocoa::base::{id, nil};
            use objc::{msg_send, sel, sel_impl};
            let wv = webview.inner() as id;
            let ns_win: id = msg_send![wv, window];
            let _: () = msg_send![ns_win, orderFront: nil];
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.show();
    }
}

pub fn do_hide_translator_window() {
    if let Some(handle) = APP_HANDLE.get() {
        match handle.get_webview_window(TRANSLATOR_WIN_NAME) {
            Some(window) => {
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = window.hide();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = tauri::AppHandle::hide(&handle);
                    let _ = window.hide();
                }
            }
            None => {}
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn hide_translator_window() {
    do_hide_translator_window();
}

// Cached physical rect of the visible thumb window. The mouse hook
// hit-tests EVERY left-button release system-wide; querying the window's
// position/size over IPC there cost three synchronous main-thread round
// trips per click (main.rs), for a 20x20 pixel comparison.
static THUMB_RECT: parking_lot::Mutex<Option<(i32, i32, i32, i32)>> = parking_lot::Mutex::new(None);

/// Zero-IPC hit test used by the mouse hook: is the screen point over the
/// (visible) thumb window?
pub fn is_thumb_hit(x: i32, y: i32) -> bool {
    match *THUMB_RECT.lock() {
        Some((x1, y1, x2, y2)) => x >= x1 && x <= x2 && y >= y1 && y <= y2,
        None => false,
    }
}

pub fn close_thumb() {
    // Already hidden: the common case after any ordinary click, and the
    // whole point of the cache — no IPC at all.
    if THUMB_RECT.lock().take().is_none() {
        return;
    }
    if let Some(handle) = APP_HANDLE.get() {
        if let Some(window) = handle.get_webview_window(THUMB_WIN_NAME) {
            let _ = window.hide();
            // Keep the hidden WebView2 from compositing (#1883).
            set_webview_visibility(&window, false);
        }
    }
}

pub fn show_thumb(x: i32, y: i32) {
    let window = get_thumb_window(x, y);
    set_webview_visibility(&window, true);
    if let Err(e) = window.show() {
        eprintln!("Error showing thumb window: {}", e);
    }
    nudge_webview_visibility(&window);
    // Cache the placed rect for the hook's hit test. We are on the
    // selection worker thread here, so the two queries are off the hot path.
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) {
        *THUMB_RECT.lock() = Some((
            position.x,
            position.y,
            position.x + size.width as i32,
            position.y + size.height as i32,
        ));
    }
}

pub fn get_thumb_window(x: i32, y: i32) -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let position_offset = 7.0 as f64;
    let window = match handle.get_webview_window(THUMB_WIN_NAME) {
        Some(window) => {
            debug_println!("Thumb window already exists");
            let _ = window.unminimize();
            let _ = window.set_always_on_top(true);
            window
        }
        None => {
            debug_println!("Thumb window does not exist");
            #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
            let mut builder = tauri::WebviewWindowBuilder::new(
                handle,
                THUMB_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .fullscreen(false)
            .focused(false)
            .inner_size(20.0, 20.0)
            .min_inner_size(20.0, 20.0)
            .max_inner_size(20.0, 20.0)
            .visible(false)
            .resizable(false)
            .skip_taskbar(true)
            .minimizable(false)
            .maximizable(false)
            .closable(false)
            .decorations(false);

            #[cfg(target_os = "windows")]
            {
                builder = builder.shadow(false);
            }

            let window = checked_build(builder);
            #[cfg(target_os = "windows")]
            {
                // use SetWindowLongPtrW in tao page to disable minimize, maximize and close buttons
                use windows::Win32::UI::WindowsAndMessaging::{
                    SetWindowLongPtrW, GWL_STYLE, WS_POPUP,
                };
                if let Ok(hwnd) = window.hwnd() {
                    // Rebuild the handle with our direct `windows` crate's
                    // HWND type: tauri links a different `windows` version, so
                    // its re-exported HWND cannot cross the type boundary.
                    let hwnd = windows::Win32::Foundation::HWND(hwnd.0);
                    unsafe {
                        // let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                        // style = style & !(0x00020000 | 0x00010000 | 0x00080000); // WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU
                        let style: u32 = WS_POPUP.0;
                        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
                    }
                }
                let _ = window.set_size(tauri::LogicalSize {
                    width: 20.0,
                    height: 20.0,
                });
            }
            post_process_window(&window);

            let _ = window.unminimize();
            let _ = window.set_always_on_top(true);

            window
        }
    };

    if cfg!(target_os = "macos") {
        let _ = window.set_position(LogicalPosition::new(
            x as f64 + position_offset,
            y as f64 + position_offset,
        ));
    } else {
        let _ = window.set_position(PhysicalPosition::new(
            x as f64 + position_offset,
            y as f64 + position_offset,
        ));
    }

    window
}

pub fn post_process_window<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    let _ = window.set_visible_on_all_workspaces(true);

    let _ = window.current_monitor();

    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::NSWindowCollectionBehavior;
        use cocoa::base::id;

        let ns_win = window.ns_window().unwrap() as id;

        unsafe {
            // Disable the automatic creation of "Show Tab Bar" etc menu items on macOS
            NSWindow::setAllowsAutomaticWindowTabbing_(ns_win, cocoa::base::NO);

            let mut collection_behavior = ns_win.collectionBehavior();
            collection_behavior |=
                NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces;

            ns_win.setCollectionBehavior_(collection_behavior);
        }
    }
}

/// Tauri/wry never propagate native window visibility to the WebView2
/// controller on Windows: a window created with `.visible(false)` — or hidden
/// later via `window.hide()` — keeps `ICoreWebView2Controller.IsVisible ==
/// TRUE`, so its renderer keeps compositing frames. For the transparent,
/// always-alive panel windows (Quick Translator, writing indicator) that
/// meant constant WebView2 CPU usage while the app sat idle in the tray
/// (#1883, #1886). Explicitly sync the controller whenever those panels are
/// created hidden, shown, or hidden again.
#[cfg(target_os = "windows")]
pub fn set_webview_visibility(window: &tauri::WebviewWindow, visible: bool) {
    let _ = window.with_webview(move |webview| unsafe {
        let _ = webview.controller().SetIsVisible(visible);
    });
}

#[cfg(not(target_os = "windows"))]
pub fn set_webview_visibility(_window: &tauri::WebviewWindow, _visible: bool) {}

// A broken WebView2 Runtime (Windows' registry says it is installed but its
// files are gone, so its own installer refuses to repair it - see discussion
// #1907) used to surface as an unwrap panic on the first webview build: the
// window frame flashed and the process aborted with 0xc0000409 before any UI
// existed, at every launch, across reinstalls. Explain and point at the
// repair download instead.
#[cfg(target_os = "windows")]
pub fn show_webview2_broken_dialog(detail: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDOK, MB_ICONERROR, MB_OKCANCEL, MB_SETFOREGROUND,
    };

    let encode = |s: &str| {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    };
    let text = encode(&format!(
        "NextAI Translator cannot start because the Microsoft Edge WebView2 Runtime on this computer is missing or broken.\n\nClick OK to open the WebView2 download page, run the installer as administrator, then start NextAI Translator again.\n\nTechnical detail: {detail}"
    ));
    let caption = encode("NextAI Translator");
    let pressed = unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_OKCANCEL | MB_ICONERROR | MB_SETFOREGROUND,
        )
    };
    if pressed == IDOK {
        let _ = std::process::Command::new("rundll32")
            .args([
                "url.dll,FileProtocolHandler",
                "https://developer.microsoft.com/microsoft-edge/webview2/",
            ])
            .spawn();
    }
}

fn handle_webview_build_failure(e: &tauri::Error) -> ! {
    #[cfg(target_os = "windows")]
    {
        show_webview2_broken_dialog(&e.to_string());
        std::process::exit(1);
    }
    #[cfg(not(target_os = "windows"))]
    panic!("failed to create webview window: {e}");
}

pub fn checked_build<R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'_, R, M>,
) -> tauri::WebviewWindow<R> {
    match builder.build() {
        Ok(window) => window,
        Err(e) => handle_webview_build_failure(&e),
    }
}

pub fn build_window<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindow<R> {
    #[cfg(target_os = "macos")]
    {
        let window = checked_build(
            builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true)
                .transparent(true),
        );

        post_process_window(&window);

        window
    }

    #[cfg(not(target_os = "macos"))]
    {
        let window = checked_build(builder.transparent(true).decorations(true));

        post_process_window(&window);

        window
    }
}

/// Opaque variant for the standard resizable utility windows (Settings,
/// History). Transparent WebView2 windows pay a heavy DirectComposition
/// cost on every resize/maximize/close on Windows, which shows up as
/// visible stutter; only the floating card/panel windows actually need
/// transparency, so these plain windows are built opaque. They are also
/// built undecorated: the pages paint their own headers (with drag
/// regions and window controls), so the native caption bar would only
/// duplicate them. Edge resizing still works - tao keeps handling the
/// resize hit-test for undecorated resizable windows.
pub fn build_opaque_window<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindow<R> {
    #[cfg(target_os = "macos")]
    {
        build_window(builder)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let window = checked_build(builder.decorations(false));

        post_process_window(&window);

        window
    }
}

#[tauri::command]
#[specta::specta]
pub async fn show_translator_window_command() {
    remember_active_window();
    show_translator_window(false, false, true);
}

/// WKWebView tracks page visibility from view/window state; after a
/// hide() -> show() cycle this can desync, leaving the page believing it
/// is hidden (rendering paused, timers throttled, process eventually
/// suspended) while the window is visibly on screen - the UI then looks
/// frozen. Toggling the view's hidden flag forces WebKit to re-evaluate.
#[cfg(target_os = "macos")]
pub fn nudge_webview_visibility(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|webview| unsafe {
        use cocoa::base::{id, NO, YES};
        use objc::{msg_send, sel, sel_impl};
        let wv = webview.inner() as id;
        let _: () = msg_send![wv, setHidden: YES];
        let _: () = msg_send![wv, setHidden: NO];
    });
}

#[cfg(not(target_os = "macos"))]
pub fn nudge_webview_visibility(_window: &tauri::WebviewWindow) {}

/// Self-healing hook for the page-side visibility watchdog: when a page
/// believes it is hidden but its window is actually (at least partially)
/// visible on screen, force WebKit to re-evaluate. WebKit's own recovery
/// notification is unreliable after occlusion/raise cycles, which left
/// pages permanently throttled (frozen rendering, crawling async work)
/// while sitting right in front of the user.
#[tauri::command]
#[specta::specta]
pub fn recover_webview_visibility(window: tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || {
            use cocoa::base::id;
            use objc::{msg_send, sel, sel_impl};
            const NS_WINDOW_OCCLUSION_STATE_VISIBLE: u64 = 1 << 1;
            if let Ok(ns_win) = w.ns_window() {
                let occlusion: u64 = unsafe { msg_send![ns_win as id, occlusionState] };
                if occlusion & NS_WINDOW_OCCLUSION_STATE_VISIBLE == 0 {
                    // The window really is fully occluded or hidden; the
                    // page's hidden state is correct.
                    return;
                }
            }
            nudge_webview_visibility(&w);
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }
}

pub fn show_translator_window(
    center: bool,
    to_mouse_position: bool,
    set_focus: bool,
) -> tauri::WebviewWindow {
    let window = get_translator_window(center, to_mouse_position, set_focus);
    if let Err(e) = window.show() {
        eprintln!("Error showing translator window: {}", e);
    }
    nudge_webview_visibility(&window);
    if set_focus {
        // An accessory app cannot reliably raise its windows above other
        // apps' windows on macOS 14+ (cooperative activation may be
        // refused), so a summoned window could stay buried under the window
        // stack - looking frozen. Temporarily float it to guarantee it
        // surfaces, then restore the user's pin preference.
        let _ = window.set_always_on_top(true);
        if !ALWAYS_ON_TOP.load(Ordering::Acquire) {
            let w = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                if !ALWAYS_ON_TOP.load(Ordering::Acquire) {
                    let _ = w.set_always_on_top(false);
                }
            });
        }
    }
    // On macOS 14+, cooperative activation can silently refuse to activate
    // an accessory app: the window is on screen but never becomes key, so
    // every click and keystroke goes nowhere and the UI appears frozen.
    // Explicitly activate the app and make the window key.
    #[cfg(target_os = "macos")]
    if set_focus {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || unsafe {
            use cocoa::appkit::NSApp;
            use cocoa::base::{id, nil, YES};
            use objc::{msg_send, sel, sel_impl};

            let app: id = NSApp();
            // The modern cooperative request first, then the legacy forced
            // variant as a fallback for systems where cooperation is refused.
            let _: () = msg_send![app, activate];
            let _: () = msg_send![app, activateIgnoringOtherApps: YES];
            if let Ok(ns_win) = w.ns_window() {
                let _: () = msg_send![ns_win as id, makeKeyAndOrderFront: nil];
            }
        });
    }
    window
}

/// Bring the translator window to the foreground.
///
/// Try SetForegroundWindow first, with AttachThreadInput as the fallback when
/// the foreground lock refuses activation after selection capture. Same trick
/// as `insertion::focus_window`.
#[cfg(target_os = "windows")]
fn focus_window_native(window: &tauri::WebviewWindow) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };

    let hwnd = match window.hwnd() {
        // Rebuild with our direct `windows` crate's HWND type: tauri links a
        // different `windows` version, so its re-exported HWND cannot cross
        // the type boundary into the imports above.
        Ok(hwnd) => windows::Win32::Foundation::HWND(hwnd.0),
        Err(e) => {
            eprintln!("Error getting translator window hwnd: {}", e);
            return;
        }
    };

    unsafe {
        if GetForegroundWindow() == hwnd || SetForegroundWindow(hwnd).as_bool() {
            return;
        }
        let foreground_thread_id = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let current_thread_id = GetCurrentThreadId();
        if foreground_thread_id == 0 || foreground_thread_id == current_thread_id {
            return;
        }
        let _ = AttachThreadInput(foreground_thread_id, current_thread_id, true);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = AttachThreadInput(foreground_thread_id, current_thread_id, false);
    }
}

#[cfg(not(target_os = "windows"))]
fn focus_window_native(window: &tauri::WebviewWindow) {
    if let Err(e) = window.set_focus() {
        eprintln!("Error focusing translator window: {}", e);
    }
}

fn focus_translator_window(window: &tauri::WebviewWindow) {
    if let Err(e) = window.unminimize() {
        eprintln!("Error unminimizing translator window: {}", e);
    }

    focus_window_native(window);

    let should_restore_on_top = !ALWAYS_ON_TOP.load(Ordering::Acquire);
    if let Err(e) = window.set_always_on_top(true) {
        eprintln!("Error enabling always on top for translator window: {}", e);
        return;
    }

    if should_restore_on_top {
        // Hold TOPMOST briefly before restoring the user's pin preference:
        // if the focus grab above was still refused (e.g. an elevated app
        // owned the foreground), an instant NOTOPMOST would let the window
        // sink straight back behind it. `show_translator_window` already
        // uses this same delayed restore.
        let w = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            if !ALWAYS_ON_TOP.load(Ordering::Acquire) {
                if let Err(e) = w.set_always_on_top(false) {
                    eprintln!(
                        "Error disabling temporary always on top for translator window: {}",
                        e
                    );
                }
            }
        });
    }
}

pub fn get_translator_window(
    center: bool,
    to_mouse_position: bool,
    set_focus: bool,
) -> tauri::WebviewWindow {
    let current_monitor = get_current_monitor();
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(TRANSLATOR_WIN_NAME) {
        Some(window) => {
            let _ = window.unminimize();
            if set_focus {
                if let Err(e) = window.set_focus() {
                    eprintln!("Error focusing translator window: {}", e);
                }
            }
            window
        }
        None => {
            let config = config::get_config_by_app(handle).unwrap_or_default();

            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                TRANSLATOR_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("NextAI Translator")
            .fullscreen(false)
            .inner_size(620.0, 700.0)
            .min_inner_size(540.0, 600.0)
            .resizable(true)
            .skip_taskbar(config.hide_the_icon_in_the_dock.unwrap_or(true))
            .visible(false)
            .focused(false);

            build_window(builder)
        }
    };

    let restore_previous_position = match config::get_config() {
        Ok(config) => config.restore_previous_position.unwrap_or(false),
        Err(e) => {
            eprintln!("Error getting config: {}", e);
            false
        }
    };

    if restore_previous_position {
        debug_println!("Restoring previous position");
        if !cfg!(target_os = "macos") {
            let _ = window.unminimize();
        }
    } else if to_mouse_position {
        debug_println!("Setting position to mouse position");
        let Ok((mouse_logical_x, mouse_logical_y)) = get_mouse_location() else {
            eprintln!("Error getting mouse location for translator window position");
            return window;
        };
        let window_physical_size = window.outer_size().unwrap_or_default();
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let mut mouse_physical_position = PhysicalPosition::new(mouse_logical_x, mouse_logical_y);
        if cfg!(target_os = "macos") {
            mouse_physical_position =
                LogicalPosition::new(mouse_logical_x as f64, mouse_logical_y as f64)
                    .to_physical(scale_factor);
        }

        let mut window_physical_position = mouse_physical_position;
        if let Some(current_monitor) = current_monitor {
            let monitor_physical_size = current_monitor.size();
            let monitor_physical_position = current_monitor.position();

            if mouse_physical_position.x + (window_physical_size.width as i32)
                > monitor_physical_position.x + (monitor_physical_size.width as i32)
            {
                window_physical_position.x = monitor_physical_position.x
                    + (monitor_physical_size.width as i32)
                    - (window_physical_size.width as i32);
            }
            if mouse_physical_position.y + (window_physical_size.height as i32)
                > monitor_physical_position.y + (monitor_physical_size.height as i32)
            {
                window_physical_position.y = monitor_physical_position.y
                    + (monitor_physical_size.height as i32)
                    - (window_physical_size.height as i32);
            }
            debug_println!("Monitor physical size: {:?}", monitor_physical_size);
            debug_println!("Monitor physical position: {:?}", monitor_physical_position);
        }
        if !cfg!(target_os = "macos") {
            let _ = window.unminimize();
        }
        debug_println!("Mouse physical position: {:?}", mouse_physical_position);
        debug_println!("Window physical size: {:?}", window_physical_size);
        debug_println!("Window physical position: {:?}", window_physical_position);
        if let Err(e) = window.set_position(window_physical_position) {
            eprintln!("Error setting translator window position: {}", e);
        }
    } else if center {
        if !cfg!(target_os = "macos") {
            let _ = window.unminimize();
        }
        let _ = window.center();
    }

    window
}

#[tauri::command]
#[specta::specta]
pub async fn show_action_manager_window() {
    let window = get_action_manager_window();
    let _ = window.center();
    if let Err(e) = window.show() {
        eprintln!("Error showing action manager window: {}", e);
    }
    nudge_webview_visibility(&window);
}

pub fn get_action_manager_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(ACTION_MANAGER_WIN_NAME) {
        Some(window) => {
            let _ = window.unminimize();
            if let Err(e) = window.set_focus() {
                eprintln!("Error focusing action manager window: {}", e);
            }
            window
        }
        None => {
            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                ACTION_MANAGER_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("NextAI Translator Action Manager")
            .fullscreen(false)
            .inner_size(700.0, 700.0)
            .min_inner_size(660.0, 600.0)
            .resizable(true)
            .skip_taskbar(true)
            .focused(true);

            return build_window(builder);
        }
    };

    window
}

#[tauri::command]
#[specta::specta]
pub async fn show_history_window() {
    let window = get_history_window();
    let _ = window.center();
    // Re-sync the controller before showing: the close handler hides the
    // window and stops the webview from compositing.
    set_webview_visibility(&window, true);
    if let Err(e) = window.show() {
        eprintln!("Error showing history window: {}", e);
    }
    nudge_webview_visibility(&window);
}

pub fn get_history_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(HISTORY_WIN_NAME) {
        Some(window) => {
            let _ = window.unminimize();
            if let Err(e) = window.set_focus() {
                eprintln!("Error focusing history window: {}", e);
            }
            window
        }
        None => {
            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                HISTORY_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("NextAI Translator History")
            .fullscreen(false)
            .inner_size(760.0, 720.0)
            .min_inner_size(660.0, 600.0)
            .resizable(true)
            .skip_taskbar(true)
            .focused(true);

            return build_opaque_window(builder);
        }
    };

    window
}

pub fn show_settings_window() {
    let window = get_settings_window();
    let _ = window.center();
    // Re-sync the controller before showing: the close handler hides the
    // window and stops the webview from compositing.
    set_webview_visibility(&window, true);
    if let Err(e) = window.show() {
        eprintln!("Error showing settings window: {}", e);
    }
    nudge_webview_visibility(&window);
}

pub fn get_settings_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(SETTINGS_WIN_NAME) {
        Some(window) => {
            let _ = window.unminimize();
            if let Err(e) = window.set_focus() {
                eprintln!("Error focusing settings window: {}", e);
            }
            window
        }
        None => {
            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                SETTINGS_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("NextAI Translator Settings")
            .fullscreen(false)
            .inner_size(660.0, 800.0)
            .min_inner_size(660.0, 600.0)
            .resizable(true)
            .skip_taskbar(true)
            .focused(true);

            return build_opaque_window(builder);
        }
    };

    window
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct CheckUpdateResultEvent(UpdateResult);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct CheckUpdateEvent;

#[tauri::command]
#[specta::specta]
pub fn show_updater_window() {
    let window = get_updater_window();
    let _ = window.center();
    if let Err(e) = window.show() {
        eprintln!("Error showing updater window: {}", e);
    }
    nudge_webview_visibility(&window);

    let handle = APP_HANDLE.get().unwrap();
    CheckUpdateEvent::listen(handle, move |event| {
        let window_clone = window.clone();
        tauri::async_runtime::spawn(async move {
            let builder = handle.updater_builder();
            let updater = match builder.build() {
                Ok(updater) => updater,
                Err(e) => {
                    eprintln!("Error building updater: {}", e);
                    return;
                }
            };

            match updater.check().await {
                Ok(Some(update)) => {
                    let _ = CheckUpdateResultEvent(UpdateResult {
                        version: update.version,
                        current_version: update.current_version,
                        body: update.body,
                    })
                    .emit(handle);
                }
                Ok(None) => {
                    let _ = handle.emit(
                        "update_result",
                        json!({
                            "result": None::<UpdateResult>
                        }),
                    );
                }
                Err(_) => {}
            }
            window_clone.unlisten(event.id)
        });
    });
}

pub fn get_updater_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(UPDATER_WIN_NAME) {
        Some(window) => {
            let _ = window.unminimize();
            if let Err(e) = window.set_focus() {
                eprintln!("Error focusing updater window: {}", e);
            }
            window
        }
        None => {
            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                UPDATER_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("NextAI Translator Updater")
            .fullscreen(false)
            .inner_size(500.0, 500.0)
            .min_inner_size(200.0, 200.0)
            .resizable(true)
            .skip_taskbar(true)
            .focused(true);

            return build_window(builder);
        }
    };

    window
}

pub fn get_inline_lookup_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    let window = match handle.get_webview_window(INLINE_LOOKUP_WIN_NAME) {
        Some(window) => {
            let _ = window.unminimize();
            window
        }
        None => {
            #[allow(unused_mut)]
            let mut builder = tauri::WebviewWindowBuilder::new(
                handle,
                INLINE_LOOKUP_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("")
            .fullscreen(false)
            .inner_size(240.0, 60.0)
            .min_inner_size(180.0, 48.0)
            .max_inner_size(500.0, 400.0)
            .visible(false)
            .resizable(true)
            .skip_taskbar(true)
            .minimizable(false)
            .maximizable(false)
            .closable(false)
            .decorations(false)
            .shadow(true);

            let window = checked_build(builder);
            post_process_window(&window);

            window
        }
    };

    window
}

pub fn show_inline_lookup_window(
    _center: bool,
    to_mouse_position: bool,
    _set_focus: bool,
) -> tauri::WebviewWindow {
    let window = get_inline_lookup_window();
    if to_mouse_position {
        if let Ok((mouse_logical_x, mouse_logical_y)) = get_mouse_location() {
            let scale_factor = window.scale_factor().unwrap_or(1.0);
            let window_physical_size = window.outer_size().unwrap_or_default();

            let mut mouse_physical_position = PhysicalPosition::new(mouse_logical_x, mouse_logical_y);
            if cfg!(target_os = "macos") {
                mouse_physical_position =
                    LogicalPosition::new(mouse_logical_x as f64, mouse_logical_y as f64)
                        .to_physical(scale_factor);
            }

            let mut window_physical_position = mouse_physical_position;

            if let Some(current_monitor) = get_current_monitor() {
                let monitor_physical_size = current_monitor.size();
                let monitor_physical_position = current_monitor.position();

                // Horizontal: clamp to right edge
                if window_physical_position.x + (window_physical_size.width as i32)
                    > monitor_physical_position.x + (monitor_physical_size.width as i32)
                {
                    window_physical_position.x = monitor_physical_position.x
                        + (monitor_physical_size.width as i32)
                        - (window_physical_size.width as i32);
                }

                // Vertical: screen divided into 6 zones
                // Flip above if bottom crosses 5/6 line, flip below if top crosses 1/6 line
                let h = monitor_physical_size.height as i32;
                let top_edge = monitor_physical_position.y;
                let zone_1_6 = top_edge + h / 6;
                let zone_5_6 = top_edge + h * 5 / 6;

                // Default: place below mouse with small gap
                let gap = 8i32;
                window_physical_position.y = mouse_physical_position.y + gap;
                let would_be_bottom =
                    window_physical_position.y + (window_physical_size.height as i32);
                if would_be_bottom > zone_5_6 {
                    // Flip above mouse with gap
                    let above_y =
                        mouse_physical_position.y - (window_physical_size.height as i32) - gap;
                    if above_y >= zone_1_6 {
                        window_physical_position.y = above_y;
                    } else {
                        window_physical_position.y = zone_1_6;
                    }
                }
                let _ = window.set_position(window_physical_position);
            }
        }
    }
    if let Err(e) = window.show() {
        eprintln!("Error showing inline lookup window: {}", e);
    }
    nudge_webview_visibility(&window);
    window
}

pub fn close_inline_lookup_window() {
    if let Some(handle) = APP_HANDLE.get() {
        if let Some(window) = handle.get_webview_window(INLINE_LOOKUP_WIN_NAME) {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn hide_inline_lookup_window() {
    close_inline_lookup_window();
}

#[tauri::command]
#[specta::specta]
pub async fn show_inline_lookup_window_command() {
    show_inline_lookup_window(false, true, true);
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn show_screenshot_window() {
    // A leftover window from a failed run has already consumed its one-shot
    // capture flow; reusing it silently does nothing (#1872), so always
    // start from a fresh window.
    let handle = APP_HANDLE.get().unwrap();
    if let Some(window) = handle.get_webview_window(SCREENSHOT_WIN_NAME) {
        let _ = window.destroy();
    }
    let _ = get_screenshot_window();
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn get_screenshot_window() -> Option<tauri::WebviewWindow> {
    let handle = APP_HANDLE.get().unwrap();
    let current_monitor = get_current_monitor()?;
    let dpi = current_monitor.scale_factor();
    let physical_position = current_monitor.position();
    let position: tauri::LogicalPosition<f64> = physical_position.to_logical(dpi);

    let window = match handle.get_webview_window(SCREENSHOT_WIN_NAME) {
        Some(window) => {
            let _ = window.set_focus();
            window
        }
        None => {
            let builder = tauri::WebviewWindowBuilder::new(
                handle,
                SCREENSHOT_WIN_NAME,
                tauri::WebviewUrl::App("src/tauri/index.html".into()),
            )
            .title("NextAI Translator Screenshot")
            .position(position.x, position.y)
            .visible(false)
            .focused(true);

            let window = build_window(builder);
            window
        }
    };

    if let Err(e) = window.set_resizable(false) {
        eprintln!("screenshot window set_resizable failed: {e:?}");
    }
    if let Err(e) = window.set_skip_taskbar(true) {
        eprintln!("screenshot window set_skip_taskbar failed: {e:?}");
    }
    #[cfg(target_os = "macos")]
    {
        let size = current_monitor.size();
        if let Err(e) = window.set_decorations(false) {
            eprintln!("screenshot window set_decorations failed: {e:?}");
        }
        if let Err(e) = window.set_size(*size) {
            eprintln!("screenshot window set_size failed: {e:?}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    if let Err(e) = window.set_fullscreen(true) {
        eprintln!("screenshot window set_fullscreen failed: {e:?}");
    }

    if let Err(e) = window.set_always_on_top(true) {
        eprintln!("screenshot window set_always_on_top failed: {e:?}");
    }

    Some(window)
}

#[cfg(target_os = "macos")]
fn apply_quick_translator_panel_traits(window: &tauri::WebviewWindow) {
    // We intentionally do NOT change the NSWindow class to NSPanel here.
    // Tao installs its own NSWindow subclass with method overrides; replacing
    // its class with NSPanel at runtime corrupts the responder chain and
    // crashes the process the next time AppKit dispatches certain selectors.
    //
    // The "doesn't steal focus" behaviour we need is already covered by:
    //   * the WebviewWindowBuilder is built with .focused(false)
    //   * we never call window.set_focus() for this window
    //   * the app activation policy is Accessory on macOS
    // What we still tweak natively here is the window level (so the panel
    // floats above the previously active app's windows) and behaviour on
    // app deactivation (don't hide when our app loses focus).
    use cocoa::base::{id, NO};
    use objc::{msg_send, sel, sel_impl};

    const NS_FLOATING_WINDOW_LEVEL: i64 = 3;

    let Some(ns_win) = window.ns_window().ok().map(|p| p as id) else {
        return;
    };

    unsafe {
        let _: () = msg_send![ns_win, setLevel: NS_FLOATING_WINDOW_LEVEL];
        let _: () = msg_send![ns_win, setHidesOnDeactivate: NO];
    }
}

#[cfg(not(target_os = "macos"))]
fn apply_quick_translator_panel_traits(_window: &tauri::WebviewWindow) {}

pub fn get_quick_translator_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    if let Some(window) = handle.get_webview_window(QUICK_TRANSLATOR_WIN_NAME) {
        let _ = window.unminimize();
        return window;
    }

    let builder = tauri::WebviewWindowBuilder::new(
        handle,
        QUICK_TRANSLATOR_WIN_NAME,
        tauri::WebviewUrl::App("src/tauri/index.html".into()),
    )
    .title("")
    .fullscreen(false)
    .inner_size(560.0, 320.0)
    .min_inner_size(420.0, 220.0)
    .max_inner_size(820.0, 620.0)
    .visible(false)
    .resizable(true)
    .skip_taskbar(true)
    .minimizable(false)
    .maximizable(false)
    .closable(false)
    .focused(false)
    .decorations(false)
    .transparent(true)
    .shadow(true);

    let window = checked_build(builder);
    post_process_window(&window);
    apply_quick_translator_panel_traits(&window);
    // Created hidden — suspend the WebView2 renderer until first show.
    set_webview_visibility(&window, false);

    #[cfg(target_os = "macos")]
    {
        use tauri::utils::config::WindowEffectsConfig;
        use tauri::utils::{WindowEffect, WindowEffectState};
        let _ = window.set_effects(WindowEffectsConfig {
            effects: vec![WindowEffect::HudWindow],
            state: Some(WindowEffectState::Active),
            radius: Some(14.0),
            color: None,
        });
    }
    window
}

fn position_quick_translator_window(window: &tauri::WebviewWindow) {
    // Anchor the Quick Translator panel to the bottom-horizontal-center of the
    // monitor that currently holds the mouse cursor, with a comfortable gap
    // from the bottom edge (Dock-aware fallback uses a fixed margin).
    let Some(current_monitor) = get_current_monitor() else {
        // No monitor (topology change): keep the window at its last position.
        return;
    };
    let window_outer_size = match window.outer_size() {
        Ok(size) => size,
        Err(_) => return,
    };

    let monitor_physical_size = current_monitor.size();
    let monitor_physical_position = current_monitor.position();
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let bottom_margin_logical = 96.0_f64;
    let bottom_margin_physical = (bottom_margin_logical * scale_factor).round() as i32;

    let x = monitor_physical_position.x
        + ((monitor_physical_size.width as i32) - (window_outer_size.width as i32)) / 2;
    let y = monitor_physical_position.y + (monitor_physical_size.height as i32)
        - (window_outer_size.height as i32)
        - bottom_margin_physical;

    let clamped_x = x.max(monitor_physical_position.x);
    let clamped_y = y.max(monitor_physical_position.y);
    let _ = window.set_position(PhysicalPosition::new(clamped_x, clamped_y));
}

pub fn show_quick_translator_window() -> tauri::WebviewWindow {
    let window = get_quick_translator_window();
    position_quick_translator_window(&window);
    set_webview_visibility(&window, true);
    let _ = window.show();
    nudge_webview_visibility(&window);
    let _ = window.set_always_on_top(true);
    if let Err(e) = APP_HANDLE
        .get()
        .map(|h| h.emit("quick-translator-shown", ()))
        .transpose()
    {
        eprintln!("failed to emit quick-translator-shown: {}", e);
    }
    window
}

#[tauri::command]
#[specta::specta]
pub async fn show_quick_translator_window_command() {
    remember_active_window();
    show_quick_translator_window();
}

#[tauri::command]
#[specta::specta]
pub async fn hide_quick_translator_window() {
    if let Some(handle) = APP_HANDLE.get() {
        if let Some(window) = handle.get_webview_window(QUICK_TRANSLATOR_WIN_NAME) {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
            set_webview_visibility(&window, false);
        }
    }
}

// ---------------------------------------------------------------------------
// Writing indicator panel
//
// A small floating, focus-less, click-through HUD anchored just below the
// currently focused input box (or the selection bounds, if a selection drove
// the writing command). Shows the "translating to <lang>" shimmer animation
// while the Writing flow streams text into the input — replacing the old
// behaviour where a placeholder like `<Translating ✍️>` was typed INTO the
// input and later backspaced away (which was the root cause of over-deletion
// when emoji grapheme widths, IME state or autocomplete made the backspace
// count wrong).
// ---------------------------------------------------------------------------

const WRITING_INDICATOR_WIDTH: f64 = 220.0;
const WRITING_INDICATOR_HEIGHT: f64 = 44.0;
const WRITING_INDICATOR_ANCHOR_GAP: f64 = 8.0;

#[cfg(target_os = "macos")]
fn apply_writing_indicator_panel_traits(window: &tauri::WebviewWindow) {
    // Same rationale as `apply_quick_translator_panel_traits`: don't reclass
    // NSWindow to NSPanel (tao's subclass breaks). Just bump the level so the
    // HUD floats above the previously active app, keep it visible when our app
    // is in the background, and make it click-through so it never accidentally
    // intercepts the user's clicks.
    use cocoa::base::{id, NO, YES};
    use objc::{class, msg_send, sel, sel_impl};

    const NS_FLOATING_WINDOW_LEVEL: i64 = 3;

    let Some(ns_win) = window.ns_window().ok().map(|p| p as id) else {
        return;
    };

    unsafe {
        let _: () = msg_send![ns_win, setLevel: NS_FLOATING_WINDOW_LEVEL];
        let _: () = msg_send![ns_win, setHidesOnDeactivate: NO];
        let _: () = msg_send![ns_win, setIgnoresMouseEvents: YES];
        // Tauri's `.transparent(true)` alone leaves a faint background in the
        // corners outside the React-rendered rounded card on some macOS
        // versions. Force the NSWindow itself to draw absolutely nothing so
        // only our rounded card is visible.
        let clear_color: id = msg_send![class!(NSColor), clearColor];
        let _: () = msg_send![ns_win, setBackgroundColor: clear_color];
        let _: () = msg_send![ns_win, setOpaque: NO];
        let _: () = msg_send![ns_win, setHasShadow: NO];
    }
}

#[cfg(not(target_os = "macos"))]
fn apply_writing_indicator_panel_traits(_window: &tauri::WebviewWindow) {}

pub fn get_writing_indicator_window() -> tauri::WebviewWindow {
    let handle = APP_HANDLE.get().unwrap();
    if let Some(window) = handle.get_webview_window(WRITING_INDICATOR_WIN_NAME) {
        return window;
    }

    let builder = tauri::WebviewWindowBuilder::new(
        handle,
        WRITING_INDICATOR_WIN_NAME,
        tauri::WebviewUrl::App("src/tauri/index.html".into()),
    )
    .title("")
    .fullscreen(false)
    .inner_size(WRITING_INDICATOR_WIDTH, WRITING_INDICATOR_HEIGHT)
    .visible(false)
    .resizable(false)
    .skip_taskbar(true)
    .minimizable(false)
    .maximizable(false)
    .closable(false)
    .focused(false)
    .decorations(false)
    .transparent(true)
    .shadow(false);

    let window = checked_build(builder);
    post_process_window(&window);
    apply_writing_indicator_panel_traits(&window);
    // Created hidden — suspend the WebView2 renderer until first show.
    set_webview_visibility(&window, false);

    // Native macOS HUD vibrancy material. With the NSWindow background set to
    // clearColor (above) the effect material is what the user actually sees
    // outside the React content — giving a true frosted-glass appearance that
    // looks at home on macOS instead of a flat CSS rgba background.
    #[cfg(target_os = "macos")]
    {
        use tauri::utils::config::WindowEffectsConfig;
        use tauri::utils::{WindowEffect, WindowEffectState};
        let _ = window.set_effects(WindowEffectsConfig {
            effects: vec![WindowEffect::HudWindow],
            state: Some(WindowEffectState::Active),
            radius: Some(14.0),
            color: None,
        });
    }

    window
}

/// Position the indicator window centered horizontally under the anchor rect
/// and `WRITING_INDICATOR_ANCHOR_GAP` pixels below it. Anchor coords are in
/// macOS logical screen points (top-left origin), matching AX coordinates.
fn position_writing_indicator_window_at_anchor(
    window: &tauri::WebviewWindow,
    anchor_x: f64,
    anchor_y: f64,
    anchor_w: f64,
    anchor_h: f64,
) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let outer = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };

    // Convert the desired logical top-left into physical pixels.
    let outer_logical_w = (outer.width as f64) / scale;
    let center_logical_x = anchor_x + anchor_w / 2.0;
    let top_logical_x = center_logical_x - outer_logical_w / 2.0;
    let top_logical_y = anchor_y + anchor_h + WRITING_INDICATOR_ANCHOR_GAP;

    let physical = LogicalPosition::new(top_logical_x, top_logical_y).to_physical::<i32>(scale);

    // Clamp to the monitor that holds the anchor's center (best effort).
    match get_current_monitor() {
        Some(monitor) => {
            let m_pos = monitor.position();
            let m_size = monitor.size();
            let max_x = m_pos.x + (m_size.width as i32) - (outer.width as i32);
            let max_y = m_pos.y + (m_size.height as i32) - (outer.height as i32);
            let x = physical.x.clamp(m_pos.x, max_x.max(m_pos.x));
            let y = physical.y.clamp(m_pos.y, max_y.max(m_pos.y));

            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
        None => {
            let _ = window.set_position(PhysicalPosition::new(physical.x, physical.y));
        }
    }
}

fn position_writing_indicator_window_near_mouse(window: &tauri::WebviewWindow) {
    let (mouse_logical_x, mouse_logical_y) = match get_mouse_location() {
        Ok(p) => p,
        Err(_) => return,
    };
    // Treat the mouse position as a 1x1 anchor so the "below + center" math
    // collapses to "below + center on cursor".
    position_writing_indicator_window_at_anchor(
        window,
        mouse_logical_x as f64,
        mouse_logical_y as f64,
        1.0,
        1.0,
    );
}

// Cached "currently-showing" target language. We mirror it in a Mutex (in
// addition to emitting the `writing-indicator-start` event) so that the React
// panel can recover the state by polling at mount time. Without this, the very
// first show often loses the event: the panel's `listen('writing-indicator-
// -start', ...)` registers asynchronously inside useEffect, and the Rust emit
// can race past it.
pub static WRITING_INDICATOR_PENDING_LANG: parking_lot::Mutex<Option<String>> =
    parking_lot::Mutex::new(None);

#[tauri::command]
#[specta::specta]
pub async fn show_writing_indicator(target_language: String) {
    debug_println!(
        "[indicator] show_writing_indicator(target_language={:?})",
        target_language
    );
    let window = get_writing_indicator_window();
    match crate::writing::peek_cached_anchor() {
        Some((x, y, w, h)) => {
            debug_println!(
                "[indicator] anchor (logical pts): ({},{},{},{})",
                x,
                y,
                w,
                h
            );
            position_writing_indicator_window_at_anchor(&window, x, y, w, h)
        }
        None => {
            debug_println!("[indicator] no AX anchor — falling back to mouse position");
            position_writing_indicator_window_near_mouse(&window);
        }
    }

    // Mirror state for the React mount-time poll.
    *WRITING_INDICATOR_PENDING_LANG.lock() = Some(target_language.clone());

    set_webview_visibility(&window, true);
    let _ = window.show();
    nudge_webview_visibility(&window);
    let _ = window.set_always_on_top(true);
    if let Some(handle) = APP_HANDLE.get() {
        // Broadcast (not emit_to). Targeted delivery to a webview that may
        // still be hidden has been observed to drop silently on some Tauri 2
        // builds; broadcast always reaches whoever is listening, and only the
        // indicator listens for this event name.
        match handle.emit(
            "writing-indicator-start",
            serde_json::json!({ "targetLanguage": target_language }),
        ) {
            Ok(_) => {
                debug_println!("[indicator] emitted writing-indicator-start");
            }
            Err(e) => {
                debug_println!("[indicator] FAILED to emit start: {:?}", e);
            }
        }
    }
}

/// Returns the target language of the currently-showing indicator, if any.
/// React calls this on mount to recover from the case where the
/// `writing-indicator-start` event was emitted before its listener was wired.
#[tauri::command]
#[specta::specta]
pub async fn get_writing_indicator_pending_lang() -> Option<String> {
    WRITING_INDICATOR_PENDING_LANG.lock().clone()
}

#[tauri::command]
#[specta::specta]
pub async fn hide_writing_indicator() {
    debug_println!("[indicator] hide_writing_indicator called");
    *WRITING_INDICATOR_PENDING_LANG.lock() = None;
    if let Some(handle) = APP_HANDLE.get() {
        if let Some(window) = handle.get_webview_window(WRITING_INDICATOR_WIN_NAME) {
            let _ = window.set_always_on_top(false);
            match window.hide() {
                Ok(_) => {
                    debug_println!("[indicator] window hidden");
                }
                Err(e) => {
                    debug_println!("[indicator] window.hide() failed: {:?}", e);
                }
            }
            set_webview_visibility(&window, false);
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an interactive Windows desktop and WebView2"]
    // Run with: cargo test --release selection_popup_ -- --ignored --test-threads=1
    fn selection_popup_stays_visible_when_pin_is_restored() {
        let mut context = tauri::generate_context!();
        context.config_mut().app.tray_icon = None;
        let app = tauri::Builder::default()
            .any_thread()
            .build(context)
            .expect("create test app");
        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "selection-visibility-regression",
            tauri::WebviewUrl::External("about:blank".parse().unwrap()),
        )
        .title("Selection visibility regression test")
        .inner_size(160.0, 100.0)
        .visible(false)
        .focused(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .data_directory(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("selection-visibility-test-profile"),
        )
        .build()
        .expect("create test window");

        // Cover first display, reopening, and restoring a minimized window,
        // both at the cursor and with the previous position retained.
        for position in [Some(PhysicalPosition::new(0, 0)), None] {
            for minimized in [false, true] {
                window.set_always_on_top(true).unwrap();
                if minimized {
                    window.minimize().unwrap();
                }
                window.hide().unwrap();
                show_selection_window(&window, position);
                window.unminimize().unwrap();
                assert!(
                    window.is_visible().unwrap(),
                    "popup must appear after capture"
                );
                assert!(!window.is_minimized().unwrap());

                // A previous summon's delayed pin restoration may arrive
                // during this summon; it must not hide the new popup.
                window.set_always_on_top(false).unwrap();
                assert!(
                    window.is_visible().unwrap(),
                    "restoring the pin preference must not hide the popup"
                );
            }
        }
        window.destroy().unwrap();
    }
}
