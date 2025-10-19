use crate::MessageHeaders;
use std::io::{Error, ErrorKind};
use std::net::Ipv4Addr;
use std::os::raw::{c_char, c_int, c_uchar, c_ushort};
use std::ptr;

#[cfg(not(feature = "tokio-runtime"))]
use std::sync::Mutex;

#[cfg(feature = "tokio-runtime")]
use tokio::sync::Mutex;

/// A TCP server that uses io_uring for I/O operations.
///
/// `Networker` wraps a C implementation that uses Linux's io_uring to handle multiple client connections.
/// The server operates in cycles, where each cycle processes pending I/O operations for all connected clients.
///
/// # Structure
///
/// The networker allocates a fixed number of client slots during initialization. Each slot can hold one
/// client connection and tracks that connection's state through the request-response lifecycle.
///
/// # Concurrency
///
/// This structure implements `Send` and `Sync`. Internal operations use a mutex to coordinate access
/// to the underlying C structures and client state.
///
/// # Features
///
/// When the `tokio-runtime` feature is enabled, methods like `cycle()` and `get_client()` become async
/// and integrate with the Tokio runtime. Without this feature, these methods are synchronous.
///
/// # Panics
///
/// Methods `cycle()` and `get_client()` panic if called on an uninitialized `Networker`.
/// Use `Networker::new()` to initialize before calling these methods. `Networker::default()`
/// creates an uninitialized instance.
///
/// # Safety
///
/// This structure wraps C FFI calls and manages raw pointers. Safety is maintained through
/// state management and the internal mutex.
///
pub struct Networker {
    primitive_self: RawNetworker,
    mutex: Mutex<()>,
    initilized: u8,
}

impl Default for Networker {
    fn default() -> Self {
        Networker {
            primitive_self: RawNetworker::default(),
            mutex: get_mutex(()),
            initilized: 0,
        }
    }
}

fn get_mutex<T>(input: T) -> Mutex<T> {
    Mutex::new(input)
}

impl Networker {
    /// The "new" function creates and initialize a new "Networker" with the given settings
    /// - Host: The IP where the networker will be listening to
    /// - Port: The port
    /// - Max_queue: The maximum count of sockets that can be left hanging before the server accepts it
    /// - Max_clients: The count of clients that will be priorly allocated
    pub fn new(host: &str, port: u16, max_queue: u16, max_clients: u16) -> Result<Self, Error> {
        let host = host.replace("localhost", "127.0.0.1");
        let valid_host = host.parse::<Ipv4Addr>().is_ok();
        if !valid_host {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid IPv4 host"));
        }
        let mut raw_net = RawNetworker::default();
        let raw_host: [i8; 16] = [0i8; 16];
        let b = host.as_bytes();
        if b.len() > 16 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid host, should be at most 16 bytes.",
            ));
        } else {
            unsafe {
                // zero-cost cast u8 -> i8
                ptr::copy_nonoverlapping(
                    b.as_ptr() as *const i8,
                    raw_host.as_ptr() as *mut i8,
                    b.len(),
                );
            }
        }
        let c = if max_clients == 0 { 1 } else { max_clients };
        let result = unsafe {
            start(
                &mut raw_net,
                &mut NetworkerSettings {
                    host: raw_host,
                    port,
                    max_queue,
                    max_clients: c,
                },
            )
        };
        if result >= 0 {
            return Ok(Networker {
                primitive_self: raw_net,
                mutex: get_mutex(()),
                initilized: 1,
            });
        }
        Err(Error::from_raw_os_error(result))
    }

    /// The networker runs in cycles, moving to the next one require this function being called
    ///
    /// I suggest you to have a loop running this during the entire program if you need full uptime
    ///
    /// # Panic
    /// It panics if you forget to initialize your networker
    #[cfg(not(feature = "tokio-runtime"))]
    pub fn cycle(&mut self) {
        if self.initilized != 1 {
            panic!("You forgot to initialize your networker :)")
        }
        unsafe { cycle(&mut self.primitive_self as *mut RawNetworker) };
    }

    #[cfg(feature = "tokio-runtime")]
    /// The networker runs in cycles, moving to the next one require this function being called
    ///
    /// I suggest you to have a loop running this during the entire program if you need full uptime
    ///
    /// # Panic
    /// It panics if you forget to initialize your networker
    pub async fn cycle(&mut self) -> Result<(), Error> {
        if self.initilized != 1 {
            panic!("You forgot to initialize your networker :)")
        }
        let pointer = (&mut self.primitive_self) as *mut RawNetworker as usize;
        let a = tokio::task::spawn_blocking(move || unsafe { cycle(pointer as *mut RawNetworker) })
            .await;
        match a {
            Ok(c) => {
                if c < 0 {
                    return Err(Error::from_raw_os_error(c));
                }
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    /// Return a client struct so you can run operations
    ///
    /// # Panic
    /// Panics if you forget to initialize your networker
    #[cfg(not(feature = "tokio-runtime"))]
    pub fn get_client(&mut self) -> Option<ClientHandler> {
        if self.initilized != 1 {
            panic!("You forgot to initialize your networker :)")
        }
        let mut _l = self.mutex.lock().unwrap();
        let a = unsafe { get_client(&mut self.primitive_self) };
        if a.exists > 0 {
            unsafe { claim_client(&mut self.primitive_self, a.client.read().id) };
            drop(_l);
            return Some(ClientHandler {
                inner: a.client,
                owner: &mut self.primitive_self,
                mutex: &mut self.mutex,
            });
        }
        None
    }
    #[cfg(feature = "tokio-runtime")]
    /// Return a client struct so you can run operations
    ///
    /// # Panic
    /// Panics if you forget to initialize your networker
    pub async fn get_client(&mut self) -> Option<ClientHandler> {
        if self.initilized != 1 {
            panic!("You forgot to initialize your networker :)")
        }
        let mut _l = self.mutex.lock().await;
        let a = unsafe { get_client(&mut self.primitive_self) };
        if a.exists > 0 {
            unsafe { claim_client(&mut self.primitive_self, a.client.read().id) };
            drop(_l);
            return Some(ClientHandler {
                inner: a.client,
                owner: &mut self.primitive_self,
                mutex: &mut self.mutex,
            });
        }
        None
    }
}

unsafe impl Sync for Networker {}
unsafe impl Send for Networker {}

// Client states
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum State {
    NonExistent = 0,
    Idle = 1,
    HeadersReaden = 2,
    FinishedH = 3,
    Reading = 4,
    FinishedR = 5,
    Available = 6,
    Processing = 7,
    Ready = 8,
    WrittingSock = 9,
    Kill = 10,
}

// Client
#[repr(C)]
struct Client {
    pub sock: c_int,
    pub request: *mut c_uchar,
    pub response: *mut c_uchar,
    pub req_headers: MessageHeaders,
    pub response_size: u64,
    pub recv_offset: usize,
    pub writev_offset: usize,
    pub id: u64,
    pub state: c_int,
    pub activity: u64,
    pub capacity: u64,
}

/// A handle to a connected client with a completed request.
///
/// `ClientHandler` provides access to a client that has received its complete request and is ready
/// for processing. The handler ensures exclusive access to the client, preventing concurrent
/// modification of the client's state.
///
/// # Lifecycle
///
/// When obtained from `Networker::get_client()`, the client transitions to a "Processing" state.
/// The client remains in this state until:
///
/// - `apply_response()` is called to send data back to the client
/// - The `ClientHandler` is dropped, which marks the client for cleanup
///
/// # Drop Behavior
///
/// When dropped without calling `apply_response()`, the client connection is terminated. This
/// prevents client-leak.
///
/// # Memory Management
///
/// The handler references the client's request buffer, which is managed by the C code. The request
/// data remains valid for the lifetime of the `ClientHandler` and becomes invalid after drop.
///
/// # Thread Safety
///
/// `ClientHandler` implements `Send` and `Sync`, allowing it to be passed between threads
/// or moved into async tasks. The internal mutex provides thread-safe access to the client's state.
pub struct ClientHandler {
    inner: *mut Client,
    owner: *mut RawNetworker,
    mutex: *mut Mutex<()>,
}
impl Drop for ClientHandler {
    #[cfg(not(feature = "tokio-runtime"))]
    fn drop(&mut self) {
        unsafe { *self.mutex.read().lock().unwrap() };
        unsafe { kill_client(self.owner, (*self.inner).id) };
    }
    #[cfg(feature = "tokio-runtime")]
    fn drop(&mut self) {
        unsafe { *self.mutex.read().blocking_lock() };
        unsafe { kill_client(self.owner, (*self.inner).id) };
    }
}

#[cfg(not(feature = "tokio-runtime"))]
impl ClientHandler {
    pub fn get_request(&self) -> (crate::CompressionAlgorithm, Vec<u8>) {
        let _lock = unsafe { (*self.mutex).lock().unwrap() };
        let mut vec: Vec<u8> =
            unsafe { Vec::with_capacity((*self.inner).req_headers.size as usize) };
        unsafe {
            ptr::copy_nonoverlapping(
                (*self.inner).request,
                vec.as_mut_ptr(),
                (*self.inner).req_headers.size as usize,
            )
        };
        unsafe { vec.set_len((*self.inner).req_headers.size as usize) };
        unsafe { ((*self.inner).req_headers.compr_alg.into(), vec) }
    }
    pub fn apply_response(
        self,
        response: Vec<u8>,
        compression_algorithm: crate::CompressionAlgorithm,
    ) -> Result<(), Error> {
        let _lock = unsafe { (*self.mutex).lock().unwrap() };
        let mut response = response;
        let res = unsafe {
            apply_client_response(
                self.owner,
                (*self.inner).id,
                response.as_mut_ptr(),
                response.len() as u64,
                compression_algorithm.into(),
            )
        };

        if res >= 0 {
            return Ok(());
        }
        return Err(Error::from_raw_os_error(res));
    }
}
#[cfg(feature = "tokio-runtime")]
impl ClientHandler {
    pub async fn get_request(&self) -> (CompressionAlgorithm, Vec<u8>) {
        let _lock = unsafe { (*self.mutex).lock().await };
        let mut vec: Vec<u8> =
            unsafe { Vec::with_capacity((*self.inner).req_headers.size as usize) };
        unsafe {
            ptr::copy_nonoverlapping(
                (*self.inner).request,
                vec.as_mut_ptr(),
                (*self.inner).req_headers.size as usize,
            )
        };
        unsafe { vec.set_len((*self.inner).req_headers.size as usize) };
        unsafe { ((*self.inner).req_headers.compr_alg.into(), vec) }
    }
    pub async fn apply_response(
        self,
        response: Vec<u8>,
        compression_algorithm: CompressionAlgorithm,
    ) -> Result<(), Error> {
        let _lock = unsafe { (*self.mutex).lock().await };
        let mut response = response;
        let res = unsafe {
            apply_client_response(
                self.owner,
                (*self.inner).id,
                response.as_mut_ptr(),
                response.len() as u64,
                compression_algorithm.into(),
            )
        };
        if res >= 0 {
            return Ok(());
        }
        Err(Error::from_raw_os_error(res))
    }
}

unsafe impl Sync for ClientHandler {}
unsafe impl Send for ClientHandler {}

// IO operations
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum Operation {
    OPSocketAcc = 0,
    OPRead = 1,
    OPWrite = 2,
    OPClose = 3,
}

// Networker settings
#[repr(C)]
pub struct NetworkerSettings {
    pub host: [c_char; 16],
    pub port: c_ushort,
    pub max_queue: c_ushort,
    pub max_clients: c_ushort,
}

// Networker
#[repr(C)]
#[derive(Default, Clone)]
pub struct RawNetworker {
    pub initiated: c_int,
    pub sock: c_int,
    pub client_num: u64,
    clients: *mut Client,
    pub ring: *mut [u8; 0],
    pub author_log: *mut u64,
}

// Rust helper struct
#[repr(C)]
pub struct SomeClient {
    client: *mut Client,
    pub exists: usize,
}

// Extern functions
#[link(name = "networker")]
unsafe extern "C" {
    fn start(self_: *mut RawNetworker, settings: *mut NetworkerSettings) -> c_int;
    fn apply_client_response(
        self_: *mut RawNetworker,
        client_id: u64,
        buffer: *const c_uchar,
        buffer_size: u64,
        compression_algorithm: c_int,
    ) -> c_int;
    fn get_client(self_: *mut RawNetworker) -> SomeClient;
    fn claim_client(self_: *mut RawNetworker, client_id: u64) -> c_int;
    fn kill_client(self_: *mut RawNetworker, client_id: u64) -> c_int;
    fn cycle(self_: *mut RawNetworker) -> c_int;
}

// test routine

#[cfg(test)]
mod networker_test {
    #[cfg(not(feature = "tokio-runtime"))]
    use std::thread;
    use std::{
        net::{IpAddr, SocketAddr},
        str::FromStr,
        thread::spawn,
        time::Duration,
    };

    #[cfg(feature = "encryption")]
    use aes_gcm::{Aes256Gcm, KeyInit};

    #[cfg(not(feature = "heuristics"))]
    use crate::CompressionAlgorithm;
    use crate::{
        falco_pipeline::{Var, pipeline_receive, pipeline_send},
        networker::Networker,
    };
    #[test]
    #[cfg(not(feature = "tokio-runtime"))]
    fn run() {
        use std::sync::{Arc, Mutex};
        use std::thread::JoinHandle;
        let var = Var {
            #[cfg(feature = "encryption")]
            cipher: Aes256Gcm::new_from_slice(&[2u8; 32]).unwrap(),
            #[cfg(not(feature = "heuristics"))]
            compression: crate::enums::CompressionAlgorithm::None,
        };
        const WORKERS: usize = 2;
        const CLIENTS: usize = 2;
        let mut js: [Option<JoinHandle<_>>; WORKERS + 1] = [const { None }; (WORKERS + 1)];
        let networker = Arc::new(Mutex::new(
            Networker::new("127.0.0.1", 8000, 128, WORKERS as u16).unwrap(),
        ));
        for i in js.iter_mut().take(WORKERS) {
            use std::thread;
            let v = var.clone();
            let networker = networker.clone();
            *i = Some(thread::spawn(move || {
                let var = v;
                loop {
                    if let Some(client) = networker.lock().unwrap().get_client() {
                        let request = client.get_request();
                        let bin = pipeline_receive(request.0.into(), request.1, &var).unwrap();
                        let response =
                            pipeline_send(bin.iter().map(|f| !f).collect(), &var).unwrap();
                        client
                            .apply_response(response.1, response.0.into())
                            .unwrap();
                    }
                    thread::yield_now();
                }
            }));
        }
        let mutexy = Arc::new(Mutex::new(CLIENTS));
        let cycle_instance = networker.clone();
        let i_mutexy = mutexy.clone();
        js[WORKERS - 1] = Some(spawn(move || {
            let mutexy = i_mutexy;
            while *mutexy.lock().unwrap() > 0 {
                cycle_instance.lock().unwrap().cycle();
                thread::yield_now();
            }
        }));

        for _ in 0..CLIENTS {
            let var = var.clone();
            let mutexy = mutexy.clone();
            spawn(move || {
                use std::ops::SubAssign;

                use crate::falco_client::FalcoClient;
                let socket = SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), 8000);
                let client = FalcoClient::new(
                    1,
                    var,
                    &socket,
                    (
                        Duration::from_secs(1),
                        Duration::from_secs(1),
                        Duration::from_secs(1),
                    ),
                    true,
                )
                .unwrap();
                client
                    .request(vec![1u8; 100])
                    .unwrap()
                    .iter()
                    .for_each(|n| assert_eq!(*n, 254));
                mutexy.lock().unwrap().sub_assign(1);
            });
        }
    }
}
