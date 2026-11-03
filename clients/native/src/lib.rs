//! One shared library, three bindings, all over `rdrand_client::hw`:
//!
//! * C ABI, used by the .NET client (P/Invoke):
//!   `int32_t rdrand_native_source(void)`  0 none, 1 RDRAND, 2 RNDR
//!   `int32_t rdrand_native_fill(uint8_t *buf, size_t len)`  0 ok, -1 no
//!   hardware generator, -2 hardware failure
//! * JNI, used by the Kotlin/JVM client: `com.jebtek.rdrand.NativeRng`
//!   `static native int source()`, `static native int fill(byte[] buf)`.
//! * Node-API, used by the Node.js client: `source()` and `fill(buffer)`.
//!   Node's `napi_*` functions are looked up in the host process when the
//!   module registers, so the library has no link-time dependency on Node
//!   and loads cleanly into .NET and Java processes too.

use rdrand_client::{hw, Source};

const OK: i32 = 0;
const UNAVAILABLE: i32 = -1;
const FAILURE: i32 = -2;

fn source_code() -> i32 {
    match hw::source() {
        None => 0,
        Some(Source::Rdrand) => 1,
        Some(Source::Rndr) => 2,
        Some(Source::Service) => 0,
    }
}

fn fill_slice(buf: &mut [u8]) -> i32 {
    match hw::fill(buf) {
        Ok(()) => OK,
        Err(hw::HwError::Unavailable) => UNAVAILABLE,
        Err(hw::HwError::Failure) => FAILURE,
    }
}

// ---- C ABI ----

#[no_mangle]
pub extern "C" fn rdrand_native_source() -> i32 {
    source_code()
}

/// # Safety
/// `buf` must be valid for writes of `len` bytes (or `len` must be 0).
#[no_mangle]
pub unsafe extern "C" fn rdrand_native_fill(buf: *mut u8, len: usize) -> i32 {
    if len == 0 {
        return OK;
    }
    if buf.is_null() {
        return FAILURE;
    }
    fill_slice(std::slice::from_raw_parts_mut(buf, len))
}

#[no_mangle]
pub extern "C" fn rdrand_native_version() -> u32 {
    20000
}

// ---- JNI ----

mod jni {
    use jni_sys::{jbyteArray, jclass, jint, JNIEnv};

    #[no_mangle]
    pub extern "system" fn Java_com_jebtek_rdrand_NativeRng_source(_env: *mut JNIEnv, _cls: jclass) -> jint {
        super::source_code()
    }

    /// # Safety
    /// Called by the JVM with a valid env and a non-null `byte[]`.
    #[no_mangle]
    pub unsafe extern "system" fn Java_com_jebtek_rdrand_NativeRng_fill(
        env: *mut JNIEnv,
        _cls: jclass,
        array: jbyteArray,
    ) -> jint {
        let f = &**env;
        let (Some(get_len), Some(set_region)) = (f.GetArrayLength, f.SetByteArrayRegion) else {
            return super::FAILURE;
        };
        let len = get_len(env, array);
        if len <= 0 {
            return super::OK;
        }
        let mut buf = vec![0u8; len as usize];
        let rc = super::fill_slice(&mut buf);
        if rc == super::OK {
            set_region(env, array, 0, len, buf.as_ptr().cast());
        }
        rc
    }
}

// ---- Node-API ----

mod napi {
    use std::ffi::{c_char, c_void, CStr};
    use std::ptr;
    use std::sync::OnceLock;

    type Env = *mut c_void;
    type Value = *mut c_void;
    type CallbackInfo = *mut c_void;
    type Status = i32;
    type Callback = unsafe extern "C" fn(Env, CallbackInfo) -> Value;

    const NAPI_OK: Status = 0;
    const AUTO_LENGTH: usize = usize::MAX;

    type CreateFunction = unsafe extern "C" fn(Env, *const c_char, usize, Callback, *mut c_void, *mut Value) -> Status;
    type SetNamedProperty = unsafe extern "C" fn(Env, Value, *const c_char, Value) -> Status;
    type GetCbInfo =
        unsafe extern "C" fn(Env, CallbackInfo, *mut usize, *mut Value, *mut Value, *mut *mut c_void) -> Status;
    type GetBufferInfo = unsafe extern "C" fn(Env, Value, *mut *mut c_void, *mut usize) -> Status;
    type CreateInt32 = unsafe extern "C" fn(Env, i32, *mut Value) -> Status;

    struct Api {
        create_function: CreateFunction,
        set_named_property: SetNamedProperty,
        get_cb_info: GetCbInfo,
        get_buffer_info: GetBufferInfo,
        create_int32: CreateInt32,
    }

    #[cfg(unix)]
    fn lookup(name: &CStr) -> *mut c_void {
        // SAFETY: dlsym with a NUL-terminated name.
        unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
    }

    #[cfg(windows)]
    fn lookup(name: &CStr) -> *mut c_void {
        extern "system" {
            fn GetModuleHandleW(name: *const u16) -> *mut c_void;
            fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        }
        // SAFETY: the host executable (node.exe) exports the napi_* functions.
        unsafe { GetProcAddress(GetModuleHandleW(ptr::null()), name.as_ptr()) }
    }

    fn api() -> Option<&'static Api> {
        static API: OnceLock<Option<Api>> = OnceLock::new();
        API.get_or_init(|| {
            macro_rules! sym {
                ($name:literal, $ty:ty) => {{
                    let p = lookup($name);
                    if p.is_null() {
                        return None;
                    }
                    // SAFETY: the symbol is Node-API's function of this name,
                    // whose C signature is `$ty`.
                    unsafe { std::mem::transmute::<*mut c_void, $ty>(p) }
                }};
            }
            Some(Api {
                create_function: sym!(c"napi_create_function", CreateFunction),
                set_named_property: sym!(c"napi_set_named_property", SetNamedProperty),
                get_cb_info: sym!(c"napi_get_cb_info", GetCbInfo),
                get_buffer_info: sym!(c"napi_get_buffer_info", GetBufferInfo),
                create_int32: sym!(c"napi_create_int32", CreateInt32),
            })
        })
        .as_ref()
    }

    unsafe fn int32(api: &Api, env: Env, v: i32) -> Value {
        let mut out = ptr::null_mut();
        if (api.create_int32)(env, v, &mut out) != NAPI_OK {
            return ptr::null_mut();
        }
        out
    }

    unsafe extern "C" fn source(env: Env, _info: CallbackInfo) -> Value {
        match api() {
            Some(api) => int32(api, env, super::source_code()),
            None => ptr::null_mut(),
        }
    }

    unsafe extern "C" fn fill(env: Env, info: CallbackInfo) -> Value {
        let Some(api) = api() else { return ptr::null_mut() };
        let mut argc = 1usize;
        let mut argv = [ptr::null_mut(); 1];
        if (api.get_cb_info)(
            env,
            info,
            &mut argc,
            argv.as_mut_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        ) != NAPI_OK
            || argc < 1
        {
            return int32(api, env, super::FAILURE);
        }
        let mut data = ptr::null_mut();
        let mut len = 0usize;
        if (api.get_buffer_info)(env, argv[0], &mut data, &mut len) != NAPI_OK {
            return int32(api, env, super::FAILURE);
        }
        let rc = if len == 0 {
            super::OK
        } else {
            super::fill_slice(std::slice::from_raw_parts_mut(data.cast::<u8>(), len))
        };
        int32(api, env, rc)
    }

    /// Node-API module entry point.
    ///
    /// # Safety
    /// Called by Node with a valid env and exports object.
    #[no_mangle]
    pub unsafe extern "C" fn napi_register_module_v1(env: Env, exports: Value) -> Value {
        let Some(api) = api() else { return exports };
        let fns: [(&CStr, Callback); 2] = [(c"source", source), (c"fill", fill)];
        for (name, cb) in fns {
            let mut f = ptr::null_mut();
            if (api.create_function)(env, name.as_ptr(), AUTO_LENGTH, cb, ptr::null_mut(), &mut f) == NAPI_OK {
                (api.set_named_property)(env, exports, name.as_ptr(), f);
            }
        }
        exports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_abi_matches_detection() {
        let mut buf = [0u8; 64];
        let rc = unsafe { rdrand_native_fill(buf.as_mut_ptr(), buf.len()) };
        if rdrand_native_source() == 0 {
            assert_eq!(rc, UNAVAILABLE);
        } else {
            assert_eq!(rc, OK);
        }
        assert_eq!(unsafe { rdrand_native_fill(std::ptr::null_mut(), 0) }, OK);
    }
}
