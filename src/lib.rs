#[cfg(windows)]
mod virtual_camera;

#[cfg(target_os = "android")]
mod android_bridge;

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
use windows::core::{GUID, HRESULT};

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    riid: *const GUID,
    result: *mut *mut c_void,
) -> HRESULT {
    unsafe { virtual_camera::dll_get_class_object(clsid, riid, result) }
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    virtual_camera::dll_can_unload_now()
}
