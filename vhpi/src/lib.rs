use bindings::*;
use std::ffi::{CStr, CString};
use std::ptr;

/// Wrapper around the C vhpiHandleT type
pub struct Handle {
    handle: vhpiHandleT,
}

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.is_null() {
            unsafe {
                vhpi_release_handle(self.handle);
            }
        }
    }
}

impl Default for Handle {
    fn default() -> Self {
        Self::null()
    }
}

impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        unsafe { vhpi_compare_handles(self.handle, other.handle) != 0 }
    }
}

impl Eq for Handle {}

impl Handle {
    pub fn null() -> Self {
        Self {
            handle: std::ptr::null_mut(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.handle.is_null()
    }

    pub fn as_raw(&self) -> vhpiHandleT {
        self.handle
    }

    pub fn from_raw(raw: vhpiHandleT) -> Self {
        Self { handle: raw }
    }
}

pub fn printf(msg: &str) {
    let cstr = CString::new(msg).expect("CString::new failed");
    unsafe { vhpi_printf(cstr.as_ptr()) };
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CbReason {
    StartOfSimulation = vhpiCbStartOfSimulation,
    EndOfSimulation = vhpiCbEndOfSimulation,
    NextTimeStep = vhpiCbNextTimeStep,
}

pub struct Callback {
    reason: CbReason,
    cb_rtn: unsafe extern "C" fn(*const vhpiCbDataS),
    obj: Handle,
    time: *mut bindings::vhpiTimeT,
    value: *mut bindings::vhpiValueT,
    user_data: *mut std::os::raw::c_void,
    flags: i32,
}

impl Callback {
    pub fn new(reason: CbReason, cb_rtn: unsafe extern "C" fn(*const vhpiCbDataS)) -> Self {
        Self {
            reason,
            cb_rtn,
            obj: Handle::null(),
            time: ptr::null_mut(),
            value: ptr::null_mut(),
            user_data: ptr::null_mut(),
            flags: 0,
        }
    }

    pub fn with_obj(mut self, obj: Handle) -> Self {
        self.obj = obj;
        self
    }

    pub fn with_time(mut self, time: *mut bindings::vhpiTimeT) -> Self {
        self.time = time;
        self
    }

    pub fn with_value(mut self, value: *mut bindings::vhpiValueT) -> Self {
        self.value = value;
        self
    }

    pub fn with_user_data(mut self, data: *mut std::os::raw::c_void) -> Self {
        self.user_data = data;
        self
    }

    pub fn with_flags(mut self, flags: i32) -> Self {
        self.flags = flags;
        self
    }

    pub fn register(self) -> Handle {
        let mut cb_data = vhpiCbDataS {
            reason: self.reason as i32,
            cb_rtn: Some(self.cb_rtn),
            obj: self.obj.as_raw(),
            time: self.time,
            value: self.value,
            user_data: self.user_data,
        };

        Handle::from_raw(unsafe { vhpi_register_cb(&mut cb_data, self.flags) })
    }
}
