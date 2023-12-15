use std::{
    ffi::{c_char, c_void, CStr, CString},
    net::SocketAddr,
    ptr::null,
    slice,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Observer {
    pub get_password: extern "C" fn(addr: *const c_char, name: *const c_char, ctx: *const c_void) -> *const c_char,
    pub allocated:
        extern "C" fn(addr: *const c_char, name: *const c_char, port: u16, ctx: *const c_void),
    pub binding: extern "C" fn(addr: *const c_char, ctx: *const c_void),
    pub channel_bind:
        extern "C" fn(addr: *const c_char, name: *const c_char, channel: u16, ctx: *const c_void),
    pub create_permission: extern "C" fn(
        addr: *const c_char,
        name: *const c_char,
        relay: *const c_char,
        ctx: *const c_void,
    ),
    pub refresh:
        extern "C" fn(addr: *const c_char, name: *const c_char, time: u32, ctx: *const c_void),
    pub abort: extern "C" fn(addr: *const c_char, name: *const c_char, ctx: *const c_void),
}

struct Delegationer {
    ctx: *const c_void,
    observer: Observer,
}

unsafe impl Sync for Delegationer {}
unsafe impl Send for Delegationer {}

impl turn_rs::Observer for Delegationer {
    /// allocate request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// In all cases, the server SHOULD only allocate ports from the range
    /// 49152 - 65535 (the Dynamic and/or Private Port range [PORT-NUMBERS]),
    /// unless the TURN server application knows, through some means not
    /// specified here, that other applications running on the same host as
    /// the TURN server application will not be impacted by allocating ports
    /// outside this range.  This condition can often be satisfied by running
    /// the TURN server application on a dedicated machine and/or by
    /// arranging that any other applications on the machine allocate ports
    /// before the TURN server application starts.  In any case, the TURN
    /// server SHOULD NOT allocate ports in the range 0 - 1023 (the Well-
    /// Known Port range) to discourage clients from using TURN to run
    /// standard services.
    fn get_password_blocking(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        let ret = (self.observer.get_password)(addr, name, self.ctx);
        drop_c_chars(&[addr, name]);
        c_char_as_str(ret).map(|item| item.to_string())
    }
    
    /// binding request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// In the Binding request/response transaction, a Binding request is
    /// sent from a STUN client to a STUN server.  When the Binding request
    /// arrives at the STUN server, it may have passed through one or more
    /// NATs between the STUN client and the STUN server (in Figure 1, there
    /// are two such NATs).  As the Binding request message passes through a
    /// NAT, the NAT will modify the source transport address (that is, the
    /// source IP address and the source port) of the packet.  As a result,
    /// the source transport address of the request received by the server
    /// will be the public IP address and port created by the NAT closest to
    /// the server.  This is called a "reflexive transport address".  The
    /// STUN server copies that source transport address into an XOR-MAPPED-
    /// ADDRESS attribute in the STUN Binding response and sends the Binding
    /// response back to the STUN client.  As this packet passes back through
    /// a NAT, the NAT will modify the destination transport address in the
    /// IP header, but the transport address in the XOR-MAPPED-ADDRESS
    /// attribute within the body of the STUN response will remain untouched.
    /// In this way, the client can learn its reflexive transport address
    /// allocated by the outermost NAT with respect to the STUN server.
    fn allocated(&self, addr: &SocketAddr, name: &str, port: u16) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.allocated)(addr, name, port, self.ctx);
        drop_c_chars(&[addr, name]);
    }

    /// binding request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// In the Binding request/response transaction, a Binding request is
    /// sent from a STUN client to a STUN server.  When the Binding request
    /// arrives at the STUN server, it may have passed through one or more
    /// NATs between the STUN client and the STUN server (in Figure 1, there
    /// are two such NATs).  As the Binding request message passes through a
    /// NAT, the NAT will modify the source transport address (that is, the
    /// source IP address and the source port) of the packet.  As a result,
    /// the source transport address of the request received by the server
    /// will be the public IP address and port created by the NAT closest to
    /// the server.  This is called a "reflexive transport address".  The
    /// STUN server copies that source transport address into an XOR-MAPPED-
    /// ADDRESS attribute in the STUN Binding response and sends the Binding
    /// response back to the STUN client.  As this packet passes back through
    /// a NAT, the NAT will modify the destination transport address in the
    /// IP header, but the transport address in the XOR-MAPPED-ADDRESS
    /// attribute within the body of the STUN response will remain untouched.
    /// In this way, the client can learn its reflexive transport address
    /// allocated by the outermost NAT with respect to the STUN server.
    fn binding(&self, addr: &SocketAddr) {
        let addr = str_to_c_char(&addr.to_string());

        (self.observer.binding)(addr, self.ctx);
        drop_c_char(addr);
    }

    /// channel binding request
    ///
    /// The server MAY impose restrictions on the IP address and port values
    /// allowed in the XOR-PEER-ADDRESS attribute; if a value is not allowed,
    /// the server rejects the request with a 403 (Forbidden) error.
    ///
    /// If the request is valid, but the server is unable to fulfill the
    /// request due to some capacity limit or similar, the server replies
    /// with a 508 (Insufficient Capacity) error.
    ///
    /// Otherwise, the server replies with a ChannelBind success response.
    /// There are no required attributes in a successful ChannelBind
    /// response.
    ///
    /// If the server can satisfy the request, then the server creates or
    /// refreshes the channel binding using the channel number in the
    /// CHANNEL-NUMBER attribute and the transport address in the XOR-PEER-
    /// ADDRESS attribute.  The server also installs or refreshes a
    /// permission for the IP address in the XOR-PEER-ADDRESS attribute as
    /// described in Section 9.
    ///
    /// NOTE: A server need not do anything special to implement
    /// idempotency of ChannelBind requests over UDP using the
    /// "stateless stack approach".  Retransmitted ChannelBind requests
    /// will simply refresh the channel binding and the corresponding
    /// permission.  Furthermore, the client must wait 5 minutes before
    /// binding a previously bound channel number or peer address to a
    /// different channel, eliminating the possibility that the
    /// transaction would initially fail but succeed on a
    /// retransmission.
    fn channel_bind(&self, addr: &SocketAddr, name: &str, num: u16) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.channel_bind)(addr, name, num, self.ctx);
        drop_c_chars(&[addr, name]);
    }

    /// create permission request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// When the server receives the CreatePermission request, it processes
    /// as per [Section 5](https://tools.ietf.org/html/rfc8656#section-5)
    /// plus the specific rules mentioned here.
    ///
    /// The message is checked for validity.  The CreatePermission request
    /// MUST contain at least one XOR-PEER-ADDRESS attribute and MAY contain
    /// multiple such attributes.  If no such attribute exists, or if any of
    /// these attributes are invalid, then a 400 (Bad Request) error is
    /// returned.  If the request is valid, but the server is unable to
    /// satisfy the request due to some capacity limit or similar, then a 508
    /// (Insufficient Capacity) error is returned.
    ///
    /// If an XOR-PEER-ADDRESS attribute contains an address of an address
    /// family that is not the same as that of a relayed transport address
    /// for the allocation, the server MUST generate an error response with
    /// the 443 (Peer Address Family Mismatch) response code.
    ///
    /// The server MAY impose restrictions on the IP address allowed in the
    /// XOR-PEER-ADDRESS attribute; if a value is not allowed, the server
    /// rejects the request with a 403 (Forbidden) error.
    ///
    /// If the message is valid and the server is capable of carrying out the
    /// request, then the server installs or refreshes a permission for the
    /// IP address contained in each XOR-PEER-ADDRESS attribute as described
    /// in [Section 9](https://tools.ietf.org/html/rfc8656#section-9).  
    /// The port portion of each attribute is ignored and may be any arbitrary
    /// value.
    ///
    /// The server then responds with a CreatePermission success response.
    /// There are no mandatory attributes in the success response.
    ///
    /// > NOTE: A server need not do anything special to implement
    /// idempotency of CreatePermission requests over UDP using the
    /// "stateless stack approach".  Retransmitted CreatePermission
    /// requests will simply refresh the permissions.
    fn create_permission(&self, addr: &SocketAddr, name: &str, relay: &SocketAddr) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);
        let relay = str_to_c_char(&relay.to_string());

        (self.observer.create_permission)(addr, name, relay, self.ctx);
        drop_c_chars(&[addr, name, relay]);
    }

    /// refresh request
    ///
    /// If the server receives a Refresh Request with a REQUESTED-ADDRESS-
    /// FAMILY attribute and the attribute value does not match the address
    /// family of the allocation, the server MUST reply with a 443 (Peer
    /// Address Family Mismatch) Refresh error response.
    ///
    /// The server computes a value called the "desired lifetime" as follows:
    /// if the request contains a LIFETIME attribute and the attribute value
    /// is zero, then the "desired lifetime" is zero.  Otherwise, if the
    /// request contains a LIFETIME attribute, then the server computes the
    /// minimum of the client's requested lifetime and the server's maximum
    /// allowed lifetime.  If this computed value is greater than the default
    /// lifetime, then the "desired lifetime" is the computed value.
    /// Otherwise, the "desired lifetime" is the default lifetime.
    ///
    /// Subsequent processing depends on the "desired lifetime" value:
    ///
    /// * If the "desired lifetime" is zero, then the request succeeds and
    /// the allocation is deleted.
    ///
    /// * If the "desired lifetime" is non-zero, then the request succeeds
    /// and the allocation's time-to-expiry is set to the "desired
    /// lifetime".
    ///
    /// If the request succeeds, then the server sends a success response
    /// containing:
    ///
    /// * A LIFETIME attribute containing the current value of the time-to-
    /// expiry timer.
    ///
    /// NOTE: A server need not do anything special to implement
    /// idempotency of Refresh requests over UDP using the "stateless
    /// stack approach".  Retransmitted Refresh requests with a non-
    /// zero "desired lifetime" will simply refresh the allocation.  A
    /// retransmitted Refresh request with a zero "desired lifetime"
    /// will cause a 437 (Allocation Mismatch) response if the
    /// allocation has already been deleted, but the client will treat
    /// this as equivalent to a success response (see below).
    fn refresh(&self, addr: &SocketAddr, name: &str, time: u32) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.refresh)(addr, name, time, self.ctx);
        drop_c_chars(&[addr, name]);
    }

    /// session abort
    ///
    /// Triggered when the node leaves from the turn. Possible reasons: the node
    /// life cycle has expired, external active deletion, or active exit of the
    /// node.
    fn abort(&self, addr: &SocketAddr, name: &str) {
        let addr = str_to_c_char(&addr.to_string());
        let name = str_to_c_char(name);

        (self.observer.abort)(addr, name, self.ctx);
        drop_c_chars(&[addr, name]);
    }
}

#[repr(C)]
pub struct Service {
    service: turn_rs::Service,
}

#[no_mangle]
pub extern "C" fn crate_turn_service(
    realm: *const c_char,
    externals_ptr: *const *const c_char,
    externals_len: usize,
    observer: Observer,
    ctx: *const c_void,
) -> *const Service {
    option_to_ptr(|| {
        let realm = c_char_as_str(realm)?.to_string();
        let mut externals = Vec::with_capacity(externals_len);
        for item in unsafe { slice::from_raw_parts(externals_ptr, externals_len) } {
            externals.push(c_char_as_str(*item)?.parse().ok()?);
        }

        Some(Box::into_raw(Box::new(Service {
            service: turn_rs::Service::new(
                realm,
                externals,
                Delegationer {
                    observer,
                    ctx,
                },
            ),
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

    match processor.processor.process_blocking(buf, addr) {
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
    }
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
