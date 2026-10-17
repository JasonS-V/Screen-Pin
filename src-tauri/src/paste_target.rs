use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TargetWindowInfo {
    pub hwnd: usize,
    pub title: String,
    pub app_name: String,
    pub is_valid: bool,
}

#[cfg(target_os = "windows")]
pub fn find_target_window_at_point(cx: i32, cy: i32) -> Option<TargetWindowInfo> {
    use std::ffi::c_void;

    type HWND = *mut c_void;
    type BOOL = i32;
    type LPARAM = isize;

    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }

    unsafe extern "system" {
        fn EnumWindows(
            lpEnumFunc: unsafe extern "system" fn(HWND, LPARAM) -> BOOL,
            lParam: LPARAM,
        ) -> BOOL;
        fn IsWindowVisible(hWnd: HWND) -> BOOL;
        fn IsIconic(hWnd: HWND) -> BOOL;
        fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
        fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
        fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut u32) -> u32;
        fn GetCurrentProcessId() -> u32;
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: BOOL, dwProcessId: u32) -> *mut c_void;
        fn CloseHandle(hObject: *mut c_void) -> BOOL;
        fn QueryFullProcessImageNameW(
            hProcess: *mut c_void,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> BOOL;
        fn GetClassNameW(hWnd: HWND, lpClassName: *mut u16, nMaxCount: i32) -> i32;
    }

    struct SearchContext {
        pt: POINT,
        my_pid: u32,
        found: Option<(HWND, String, String)>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let ctx = &mut *(lparam as *mut SearchContext);

            if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return 1; // Continuar
            }

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == ctx.my_pid || pid == 0 {
                return 1; // Ignorar ventanas de nuestra propia aplicación o del sistema
            }

            // Ignorar ventanas de la barra de tareas y el fondo de escritorio
            let mut class_buf = [0u16; 128];
            let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 128);
            if class_len > 0 {
                let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
                if class_name == "Shell_TrayWnd"
                    || class_name == "Progman"
                    || class_name == "WorkerW"
                    || class_name == "Windows.UI.Core.CoreWindow"
                {
                    return 1;
                }
            }

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut rect) != 0 {
                if rect.right <= rect.left || rect.bottom <= rect.top {
                    return 1;
                }

                if ctx.pt.x >= rect.left
                    && ctx.pt.x <= rect.right
                    && ctx.pt.y >= rect.top
                    && ctx.pt.y <= rect.bottom
                {
                    // Encontrada ventana superior visible que contiene el punto
                    let mut title_buf = [0u16; 256];
                    let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
                    let title = if title_len > 0 {
                        String::from_utf16_lossy(&title_buf[..title_len as usize]).trim().to_string()
                    } else {
                        String::new()
                    };

                    let mut exe_name = String::new();
                    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
                    let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                    if !h_proc.is_null() {
                        let mut exe_buf = [0u16; 512];
                        let mut size = 512u32;
                        if QueryFullProcessImageNameW(h_proc, 0, exe_buf.as_mut_ptr(), &mut size) != 0 {
                            let path = String::from_utf16_lossy(&exe_buf[..size as usize]);
                            if let Some(stem) = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()) {
                                exe_name = stem.to_string();
                            }
                        }
                        CloseHandle(h_proc);
                    }

                    let friendly_app = format_friendly_app_name(&exe_name, &title);
                    ctx.found = Some((hwnd, title, friendly_app));
                    return 0; // Detener enumeración
                }
            }

            1 // Continuar
        }
    }

    unsafe {
        let my_pid = GetCurrentProcessId();
        let mut ctx = SearchContext {
            pt: POINT { x: cx, y: cy },
            my_pid,
            found: None,
        };

        EnumWindows(enum_proc, &mut ctx as *mut _ as LPARAM);

        if let Some((hwnd, title, app_name)) = ctx.found {
            Some(TargetWindowInfo {
                hwnd: hwnd as usize,
                title,
                app_name,
                is_valid: true,
            })
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn find_target_window_at_point(_cx: i32, _cy: i32) -> Option<TargetWindowInfo> {
    None
}

pub fn format_friendly_app_name(exe: &str, title: &str) -> String {
    let lower_exe = exe.to_lowercase();
    match lower_exe.as_str() {
        "chrome" => "Chrome".into(),
        "msedge" => "Edge".into(),
        "firefox" => "Firefox".into(),
        "brave" => "Brave".into(),
        "code" => "VS Code".into(),
        "discord" => "Discord".into(),
        "whatsapp" => "WhatsApp".into(),
        "telegram" => "Telegram".into(),
        "slack" => "Slack".into(),
        "winword" => "Word".into(),
        "excel" => "Excel".into(),
        "powerpnt" => "PowerPoint".into(),
        "notepad" => "Bloc de notas".into(),
        "mspaint" => "Paint".into(),
        "explorer" => "Explorador".into(),
        "figma" => "Figma".into(),
        "photoshop" => "Photoshop".into(),
        "obsidian" => "Obsidian".into(),
        "notion" => "Notion".into(),
        "spotify" => "Spotify".into(),
        "teams" | "msteams" => "Teams".into(),
        _ => {
            if !title.is_empty() {
                if let Some(pos) = title.rfind(" - ") {
                    let suffix = title[pos + 3..].trim();
                    if !suffix.is_empty() && suffix.len() <= 20 {
                        return suffix.to_string();
                    }
                }
            }
            if !exe.is_empty() {
                let mut chars = exe.chars();
                match chars.next() {
                    None => "otra aplicación".into(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            } else {
                "esta ventana".into()
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub async fn send_paste_to_target_hwnd(target_hwnd: usize) -> Result<(), String> {
    use std::ffi::c_void;
    type HWND = *mut c_void;
    type BOOL = i32;

    unsafe extern "system" {
        fn IsWindow(hWnd: HWND) -> BOOL;
        fn GetForegroundWindow() -> HWND;
        fn SetForegroundWindow(hWnd: HWND) -> BOOL;
        fn SetFocus(hWnd: HWND) -> HWND;
        fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
        fn AttachThreadInput(idAttach: u32, idAttachTo: u32, fAttach: BOOL) -> BOOL;
        fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut u32) -> u32;
        fn GetCurrentThreadId() -> u32;
        fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    }

    let hwnd = target_hwnd as HWND;
    unsafe {
        if IsWindow(hwnd) == 0 {
            return Err("La ventana destino ya no existe".into());
        }

        let cur_fg = GetForegroundWindow();
        let cur_thread = GetCurrentThreadId();
        let fg_thread = GetWindowThreadProcessId(cur_fg, std::ptr::null_mut());
        let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());

        if cur_thread != target_thread {
            AttachThreadInput(cur_thread, target_thread, 1);
        }
        if fg_thread != target_thread {
            AttachThreadInput(fg_thread, target_thread, 1);
        }

        ShowWindow(hwnd, 9); // SW_RESTORE
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);

        if cur_thread != target_thread {
            AttachThreadInput(cur_thread, target_thread, 0);
        }
        if fg_thread != target_thread {
            AttachThreadInput(fg_thread, target_thread, 0);
        }
    }

    // Pequeño retardo para que Windows procese el cambio de foco de entrada
    std::thread::sleep(std::time::Duration::from_millis(90));

    unsafe {
        // Enviar simulación de Ctrl + V
        const VK_CONTROL: u8 = 0x11;
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        keybd_event(VK_CONTROL, 0, 0, 0);
        keybd_event(b'V', 0, 0, 0);
        keybd_event(b'V', 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub async fn send_paste_to_target_hwnd(_target_hwnd: usize) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_friendly_app_name() {
        assert_eq!(format_friendly_app_name("chrome", "New Tab - Google Chrome"), "Chrome");
        assert_eq!(format_friendly_app_name("msedge", "Microsoft Edge"), "Edge");
        assert_eq!(format_friendly_app_name("teams", "Microsoft Teams"), "Teams");
        assert_eq!(format_friendly_app_name("code", "PinView.svelte - Screen Pin"), "VS Code");
        assert_eq!(format_friendly_app_name("notepad", "Untitled - Notepad"), "Bloc de notas");
        assert_eq!(format_friendly_app_name("unknown_app", "Doc - CustomApp"), "CustomApp");
        assert_eq!(format_friendly_app_name("calc", ""), "Calc");
        assert_eq!(format_friendly_app_name("", ""), "esta ventana");
    }
}
