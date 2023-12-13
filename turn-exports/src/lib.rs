use std::{
    ffi::{c_char, c_void, CStr, CString},
    net::SocketAddr,
    ptr::null,
    slice,
    sync::Arc,
};

use async_trait::async_trait;
use tokio::{
    runtime::Runtime,
    sync::oneshot::{self, Sender},
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Observer {
    pub get_password: extern "C" fn(
        addr: *const c_char,
        name: *const c_char,
        callback: extern "C" fn(ret: *const c_char, ctx: *const c_void),
        ctx: *const c_void,
    ),
    pub allocated: extern "C" fn(addr: *const c_char, name: *const c_char, port: u16),
    pub binding: extern "C" fn(addr: *const c_char),
    pub channel_bind: extern "C" fn(addr: *const c_char, name: *const c_char, channel: u16),
    pub create_permission:
        extern "C" fn(addr: *const c_char, name: *const c_char, relay: *const c_char),
    pub refresh: extern "C" fn(addr: *const c_char, name: *const c_char, time: u32),
    pub abort: extern "C" fn(addr: *const c_char, name: *const c_char),
}

struct Delegationer {
    observer: Observer,
    runtime: Arc<Runtime>,
}

#[async_trait]
impl turn_rs::Observer for Delegationer {
    async fn get_password(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        let (tx, rx) = oneshot::channel::<Option<String>>();
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.get_password)(
            addr,
            name,
            get_password_callback,
            Box::into_raw(Box::new(tx)) as *const c_void,
        );

        drop_c_chars(&[addr, name]);
        self.runtime.spawn(rx).await.ok()?.ok()?
    }

    fn allocated(&self, addr: &SocketAddr, name: &str, port: u16) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.allocated)(addr, name, port);
        drop_c_chars(&[addr, name]);
    }

    fn binding(&self, addr: &SocketAddr) {
        let addr = str_to_c_char(&addr.to_string());

        (self.observer.binding)(addr);
        drop_c_char(addr);
    }

    fn channel_bind(&self, addr: &SocketAddr, name: &str, num: u16) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.channel_bind)(addr, name, num);
        drop_c_chars(&[addr, name]);
    }

    fn create_permission(&self, addr: &SocketAddr, name: &str, relay: &SocketAddr) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);
        let relay = str_to_c_char(&relay.to_string());

        (self.observer.create_permission)(addr, name, relay);
        drop_c_chars(&[addr, name, relay]);
    }

    fn refresh(&self, addr: &SocketAddr, name: &str, time: u32) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.refresh)(addr, name, time);
        drop_c_chars(&[addr, name]);
    }

    fn abort(&self, addr: &SocketAddr, name: &str) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.abort)(addr, name);
        drop_c_chars(&[addr, name]);
    }
}

extern "C" fn get_password_callback(ret: *const c_char, ctx: *const c_void) {
    let sender = unsafe { Box::from_raw(ctx as *mut Sender<Option<String>>) };
    let _ = sender.send(if !ret.is_null() {
        if let Some(value) = c_char_as_str(ret) {
            Some(value.to_string())
        } else {
            None
        }
    } else {
        None
    });
}

#[repr(C)]
pub struct Service {
    service: turn_rs::Service,
    runtime: Arc<Runtime>,
}

#[no_mangle]
pub extern "C" fn crate_turn_service(
    realm: *const c_char,
    externals_ptr: *const *const c_char,
    externals_len: usize,
    observer: Observer,
) -> *const Service {
    option_to_ptr(|| {
        let realm = c_char_as_str(realm)?.to_string();
        let mut externals = Vec::with_capacity(externals_len);
        for item in unsafe { slice::from_raw_parts(externals_ptr, externals_len) } {
            externals.push(c_char_as_str(*item)?.parse().ok()?);
        }

        let runtime = Arc::new(Runtime::new().ok()?);
        Some(Box::into_raw(Box::new(Service {
            service: turn_rs::Service::new(
                realm,
                externals,
                Delegationer {
                    runtime: runtime.clone(),
                    observer,
                },
            ),
            runtime,
        })))
    })
}

#[no_mangle]
pub extern "C" fn drop_turn_service(service: *const Service) {
    assert!(!service.is_null());

    drop(unsafe { Box::from_raw(service as *mut Observer) })
}

#[repr(C)]
pub struct Processor {
    processor: turn_rs::Processor,
    runtime: Arc<Runtime>,
}

#[no_mangle]
pub extern "C" fn get_processor(
    service: *const Service,
    interface: *const c_char,
    external: *const c_char,
) -> *mut Processor {
    assert!(!service.is_null());

    option_to_ptr(|| {
        Some(Box::into_raw(Box::new(Processor {
            runtime: unsafe { &(*service) }.runtime.clone(),
            processor: unsafe { &(*service) }.service.get_processor(
                c_char_as_str(interface)?.parse().ok()?,
                c_char_as_str(external)?.parse().ok()?,
            ),
        })))
    }) as *mut _
}

#[no_mangle]
pub extern "C" fn drop_processor(processor: *mut Processor) {
    assert!(!processor.is_null());

    drop(unsafe { Box::from_raw(processor) })
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Response {
    pub data: *const u8,
    pub data_len: usize,
    pub kind: turn_rs::StunClass,
    pub relay: *const c_char,
    pub interface: *const c_char,
}

#[repr(C)]
pub union ProcessResult {
    pub response: Response,
    pub error: faster_stun::StunError,
}

#[no_mangle]
pub extern "C" fn process(
    processor: *mut Processor,
    buf: *const u8,
    buf_len: usize,
    addr: *const c_char,
    callback: extern "C" fn(is_success: bool, ret: *const ProcessResult, ctx: *const c_void),
    ctx: *const c_void,
) {
    assert!(!processor.is_null());

    let addr: SocketAddr = if let Some(Ok(addr)) = c_char_as_str(addr).map(|item| item.parse()) {
        addr
    } else {
        return (callback)(false, null(), ctx);
    };

    let processor = unsafe { &mut (*processor) };
    let buf = unsafe { slice::from_raw_parts(buf, buf_len) };
    let ctx = ctx as usize;

    processor.runtime.clone().spawn(async move {
        match processor.processor.process(buf, addr).await {
            Err(e) => (callback)(false, &ProcessResult { error: e }, ctx as *const c_void),
            Ok(ret) => {
                if let Some(ret) = ret {
                    let relay = ret
                        .relay
                        .map(|item| str_to_c_char(&item.to_string()))
                        .unwrap_or_else(|| null());
                    let interface = ret
                        .interface
                        .map(|item| str_to_c_char(&item.to_string()))
                        .unwrap_or_else(|| null());

                    (callback)(
                        true,
                        &ProcessResult {
                            response: Response {
                                kind: ret.kind,
                                data: ret.data.as_ptr(),
                                data_len: ret.data.len(),
                                interface,
                                relay,
                            },
                        },
                        ctx as *const c_void,
                    );

                    drop_c_chars(&[relay, interface]);
                } else {
                    (callback)(false, null(), ctx as *const c_void)
                }
            }
        };
    });
}

fn c_char_as_str(input: *const c_char) -> Option<&'static str> {
    if input.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(input) }.to_str().ok()
    }
}

fn str_to_c_char(input: &str) -> *const c_char {
    CString::new(input).unwrap().into_raw()
}

fn drop_c_char(input: *const c_char) {
    if !input.is_null() {
        drop(unsafe { CString::from_raw(input as *mut c_char) })
    }
}

fn drop_c_chars(input: &[*const c_char]) {
    for item in input {
        drop_c_char(*item);
    }
}

fn option_to_ptr<F, T>(func: F) -> *const T
where
    F: Fn() -> Option<*const T>,
{
    match func() {
        None => null(),
        Some(ptr) => ptr,
    }
}
