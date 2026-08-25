use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::{self, NonNull};

const MPV_FORMAT_FLAG: c_int = 3;
const MPV_FORMAT_DOUBLE: c_int = 5;
const MPV_ERROR_PROPERTY_UNAVAILABLE: c_int = -10;

#[repr(C)]
pub struct mpv_handle {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn mpv_create() -> *mut mpv_handle;
    fn mpv_initialize(ctx: *mut mpv_handle) -> c_int;
    fn mpv_terminate_destroy(ctx: *mut mpv_handle);
    fn mpv_error_string(error: c_int) -> *const c_char;
    fn mpv_set_option_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn mpv_command(ctx: *mut mpv_handle, args: *const *const c_char) -> c_int;
    fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    fn mpv_get_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
}

pub struct MpvHandle {
    raw: NonNull<mpv_handle>,
}

impl MpvHandle {
    pub fn new_with_keep_open(keep_open: bool) -> els_types::AppResult<Self> {
        Self::set_mpv_numeric_locale();
        let raw = NonNull::new(unsafe { mpv_create() }).ok_or_else(|| {
            els_types::AppError::Io("libmpv failed to create player handle".to_string())
        })?;

        let handle = Self { raw };
        handle.set_option_string("config", "no")?;
        handle.set_option_string("vo", "libmpv")?;
        handle.set_option_string("terminal", "no")?;
        handle.set_option_string("keep-open", if keep_open { "yes" } else { "no" })?;
        handle.set_option_string("idle", "yes")?;
        handle.set_option_string("pause", "yes")?;
        handle.set_option_string("keepaspect", "yes")?;
        handle.set_option_string("panscan", "0.0")?;
        handle.set_option_string("sub-auto", "no")?;
        handle.set_option_string("sid", "no")?;
        handle.check_status(
            unsafe { mpv_initialize(handle.raw.as_ptr()) },
            "initialize libmpv",
        )?;
        Ok(handle)
    }

    fn set_mpv_numeric_locale() {
        std::env::set_var("LC_NUMERIC", "C");
        unsafe {
            #[cfg(not(target_os = "windows"))]
            libc::setenv(c"LC_NUMERIC".as_ptr(), c"C".as_ptr(), 1);
            libc::setlocale(libc::LC_NUMERIC, c"C".as_ptr());
        }
    }

    pub fn raw_handle_value(&self) -> u64 {
        self.raw.as_ptr() as usize as u64
    }

    pub fn load_file(&self, path: &str) -> els_types::AppResult<()> {
        let command = [
            CString::new("loadfile").expect("static command"),
            to_cstring(path)?,
            CString::new("replace").expect("static command"),
        ];
        self.command(&command)
    }

    pub fn set_pause(&self, paused: bool) -> els_types::AppResult<()> {
        let name = CString::new("pause").expect("static property");
        let mut value: c_int = if paused { 1 } else { 0 };
        self.check_status(
            unsafe {
                mpv_set_property(
                    self.raw.as_ptr(),
                    name.as_ptr(),
                    MPV_FORMAT_FLAG,
                    (&mut value as *mut c_int).cast::<c_void>(),
                )
            },
            "set pause property",
        )
    }

    pub fn seek_absolute(&self, position_secs: f64) -> els_types::AppResult<()> {
        let name = CString::new("time-pos").expect("static property");
        let mut value = position_secs;
        self.check_status(
            unsafe {
                mpv_set_property(
                    self.raw.as_ptr(),
                    name.as_ptr(),
                    MPV_FORMAT_DOUBLE,
                    (&mut value as *mut f64).cast::<c_void>(),
                )
            },
            "set time-pos property",
        )
    }

    pub fn set_speed(&self, playback_rate: f64) -> els_types::AppResult<()> {
        let name = CString::new("speed").expect("static property");
        let mut value = playback_rate;
        self.check_status(
            unsafe {
                mpv_set_property(
                    self.raw.as_ptr(),
                    name.as_ptr(),
                    MPV_FORMAT_DOUBLE,
                    (&mut value as *mut f64).cast::<c_void>(),
                )
            },
            "set speed property",
        )
    }

    pub fn time_pos(&self) -> els_types::AppResult<Option<f64>> {
        self.get_optional_double("time-pos")
    }

    pub fn duration(&self) -> els_types::AppResult<Option<f64>> {
        self.get_optional_double("duration")
    }

    pub fn paused(&self) -> els_types::AppResult<Option<bool>> {
        let name = CString::new("pause").expect("static property");
        let mut value: c_int = 0;
        let status = unsafe {
            mpv_get_property(
                self.raw.as_ptr(),
                name.as_ptr(),
                MPV_FORMAT_FLAG,
                (&mut value as *mut c_int).cast::<c_void>(),
            )
        };

        if status == MPV_ERROR_PROPERTY_UNAVAILABLE {
            return Ok(None);
        }

        self.check_status(status, "get pause property")?;
        Ok(Some(value != 0))
    }

    fn get_optional_double(&self, property_name: &str) -> els_types::AppResult<Option<f64>> {
        let name = to_cstring(property_name)?;
        let mut value = 0.0_f64;
        let status = unsafe {
            mpv_get_property(
                self.raw.as_ptr(),
                name.as_ptr(),
                MPV_FORMAT_DOUBLE,
                (&mut value as *mut f64).cast::<c_void>(),
            )
        };

        if status == MPV_ERROR_PROPERTY_UNAVAILABLE {
            return Ok(None);
        }

        self.check_status(status, &format!("get {property_name} property"))?;
        Ok(Some(value))
    }

    fn set_option_string(&self, option_name: &str, value: &str) -> els_types::AppResult<()> {
        let option_name = to_cstring(option_name)?;
        let value = to_cstring(value)?;
        self.check_status(
            unsafe {
                mpv_set_option_string(self.raw.as_ptr(), option_name.as_ptr(), value.as_ptr())
            },
            "set libmpv option",
        )
    }

    fn command(&self, args: &[CString]) -> els_types::AppResult<()> {
        let mut pointers = args.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();
        pointers.push(ptr::null());
        self.check_status(
            unsafe { mpv_command(self.raw.as_ptr(), pointers.as_ptr()) },
            "run libmpv command",
        )
    }

    fn check_status(&self, status: c_int, action: &str) -> els_types::AppResult<()> {
        if status >= 0 {
            Ok(())
        } else {
            Err(els_types::AppError::Io(format!(
                "{action}: {}",
                error_string(status)
            )))
        }
    }
}

impl Drop for MpvHandle {
    fn drop(&mut self) {
        unsafe {
            mpv_terminate_destroy(self.raw.as_ptr());
        }
    }
}

fn to_cstring(value: &str) -> els_types::AppResult<CString> {
    CString::new(value).map_err(|_| {
        els_types::AppError::InvalidArgument("libmpv input contained a null byte".to_string())
    })
}

fn error_string(status: c_int) -> String {
    unsafe {
        let ptr = mpv_error_string(status);
        if ptr.is_null() {
            format!("unknown libmpv error ({status})")
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}
