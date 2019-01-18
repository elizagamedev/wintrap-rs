use std::ffi::OsString;
use std::os::windows::prelude::*;
use std::{mem, ptr, slice};
use winapi::shared::minwindef::{
    ATOM, BOOL, DWORD, FALSE, HINSTANCE, HLOCAL, HMODULE, LPARAM, LRESULT, TRUE, UINT, WPARAM,
};
use winapi::shared::ntdef::{LANG_NEUTRAL, LPCWSTR, LPWSTR, MAKELANGID, SUBLANG_DEFAULT};
use winapi::shared::windef::HWND;
use winapi::um::consoleapi::SetConsoleCtrlHandler;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winbase::{
    FormatMessageW, LocalFree, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
    FORMAT_MESSAGE_IGNORE_INSERTS,
};
use winapi::um::winuser::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, EnumWindows, GetMessageW,
    GetWindowThreadProcessId, PostMessageW, PostThreadMessageW, RegisterClassExW,
    RegisterWindowMessageW, TranslateMessage, UnregisterClassW, MSG, WNDCLASSEXW,
};

lazy_static! {
    static ref WINDOW_IDENTIFIER: Vec<u16> = OsString::from("RUST_WINSIG")
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>();
}

pub fn get_last_error() -> DWORD {
    unsafe { GetLastError() }
}

pub fn set_console_ctrl_handler(
    handler_routine: unsafe extern "system" fn(CtrlType: DWORD) -> BOOL,
    add: bool,
) -> Result<(), DWORD> {
    let result =
        unsafe { SetConsoleCtrlHandler(Some(handler_routine), if add { TRUE } else { FALSE }) };
    if result == FALSE {
        Err(get_last_error())
    } else {
        Ok(())
    }
}

pub fn get_module_handle() -> Result<HMODULE, DWORD> {
    let result = unsafe { GetModuleHandleW(ptr::null()) };
    if result.is_null() {
        Err(get_last_error())
    } else {
        Ok(result)
    }
}

pub fn register_window_message(identifier: impl AsRef<str>) -> Result<UINT, DWORD> {
    let identifier_wide = OsString::from(identifier.as_ref())
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>();
    let result = unsafe { RegisterWindowMessageW(identifier_wide.as_ptr()) };
    if result == 0 {
        Err(get_last_error())
    } else {
        Ok(result)
    }
}

pub fn post_thread_message(
    thread_id: u32,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Result<(), DWORD> {
    let result = unsafe { PostThreadMessageW(thread_id, msg, wparam, lparam) };
    if result == FALSE {
        Err(get_last_error())
    } else {
        Ok(())
    }
}

pub fn post_message(
    window_handle: WindowHandle,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Result<(), DWORD> {
    let result = unsafe { PostMessageW(window_handle.hwnd, msg, wparam, lparam) };
    if result == FALSE {
        Err(get_last_error())
    } else {
        Ok(())
    }
}

pub fn get_current_thread_id() -> DWORD {
    unsafe { GetCurrentThreadId() }
}

pub fn enum_windows(proc: unsafe extern "system" fn(HWND, LPARAM) -> BOOL, lparam: LPARAM) -> bool {
    unsafe { EnumWindows(Some(proc), lparam) != FALSE }
}

pub fn get_window_thread_process_id(hwnd: HWND) -> (DWORD, DWORD) {
    unsafe {
        let mut process_id: DWORD = mem::uninitialized();
        let thread_id = GetWindowThreadProcessId(hwnd, &mut process_id as *mut DWORD);
        (thread_id, process_id)
    }
}

pub fn format_error(code: DWORD) -> Result<String, DWORD> {
    let mut buf: LPWSTR = unsafe { mem::uninitialized() };
    #[allow(clippy::crosspointer_transmute)]
    let buf_ptr = unsafe { mem::transmute::<*mut LPWSTR, LPWSTR>(&mut buf as *mut LPWSTR) };
    let size = unsafe {
        FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER
                | FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_IGNORE_INSERTS,
            ptr::null_mut(),
            code,
            DWORD::from(MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT)),
            buf_ptr,
            0,
            ptr::null_mut(),
        )
    };
    if size == 0 {
        return Err(get_last_error());
    }
    let wide =
        OsString::from_wide(unsafe { slice::from_raw_parts(buf as *const u16, size as usize) });
    unsafe { LocalFree(buf as HLOCAL) };
    let mut result = wide.into_string().unwrap();
    result.truncate(result.trim().len());
    Ok(result)
}

pub struct Window {
    pub hwnd: HWND,
    #[used]
    window_class: WindowClass,
}

impl Window {
    pub fn new(
        proc: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
    ) -> Result<Window, DWORD> {
        let instance = get_module_handle()?;
        let window_class = WindowClass::new(proc, instance)?;
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                window_class.wndclass as LPCWSTR,
                WINDOW_IDENTIFIER.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null_mut(),
            );
            if hwnd.is_null() {
                Err(get_last_error())
            } else {
                Ok(Window { hwnd, window_class })
            }
        }
    }

    pub fn run_event_loop(&mut self, message_cb: impl Fn(&MSG)) -> Result<i32, DWORD> {
        loop {
            let mut msg: MSG = unsafe { mem::uninitialized() };
            let ret = unsafe { GetMessageW(&mut msg as *mut MSG, ptr::null_mut(), 0, 0) };
            if ret > 0 {
                message_cb(&msg);
                unsafe {
                    TranslateMessage(&mut msg as *mut MSG);
                    DispatchMessageW(&mut msg as *mut MSG);
                }
            } else if ret < 0 {
                return Err(get_last_error());
            } else {
                return Ok(msg.wParam as i32);
            }
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

unsafe impl Sync for Window {}

unsafe impl Send for Window {}

struct WindowClass {
    wndclass: ATOM,
    instance: HINSTANCE,
}

impl WindowClass {
    fn new(
        proc: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
        instance: HMODULE,
    ) -> Result<WindowClass, DWORD> {
        let mut opts = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null_mut(),
            lpszClassName: WINDOW_IDENTIFIER.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        let wndclass = unsafe { RegisterClassExW(&mut opts as *mut WNDCLASSEXW) };
        if wndclass == 0 {
            Err(unsafe { GetLastError() })
        } else {
            Ok(WindowClass { wndclass, instance })
        }
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        unsafe { UnregisterClassW(self.wndclass as *mut u16, self.instance) };
    }
}

#[derive(Copy, Clone)]
pub struct WindowHandle {
    pub hwnd: HWND,
}

unsafe impl Sync for WindowHandle {}

unsafe impl Send for WindowHandle {}
