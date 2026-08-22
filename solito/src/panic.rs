use std::ffi::OsStr;
use std::io::Write;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::{io, panic, ptr};

use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TASKMODAL, MessageBoxW,
};

// Install a panic handler that renders the panic in a classical Windows error
// dialog box as well as writes the panic to STDERR.
pub fn attach_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let _ = writeln!(io::stderr(), "{}", panic_info);
        let msg = format!("{}\n\nPress Ctrl-C to Copy", panic_info);
        unsafe {
            MessageBoxW(
                ptr::null_mut(),
                win32_string(&msg).as_ptr(),
                win32_string("Solito: Runtime Error").as_ptr(),
                MB_ICONERROR | MB_OK | MB_SETFOREGROUND | MB_TASKMODAL,
            );
        }
    }));
}

/// Converts the string slice into a Windows-standard representation for "W"-
/// suffixed function variants, which accept UTF-16 encoded string values.
fn win32_string<S: AsRef<OsStr> + ?Sized>(value: &S) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}
