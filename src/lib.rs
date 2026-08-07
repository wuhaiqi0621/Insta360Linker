mod virtual_camera;

use std::ffi::c_void;

use windows::core::{GUID, HRESULT};

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    riid: *const GUID,
    result: *mut *mut c_void,
) -> HRESULT {
    unsafe { virtual_camera::dll_get_class_object(clsid, riid, result) }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    virtual_camera::dll_can_unload_now()
}
