use libc::c_longlong;
use std::io::{Error, ErrorKind};
use std::mem;
use std::net::Ipv4Addr;
use std::os::raw::{c_char, c_int, c_uchar, c_ushort};
use std::ptr;
use std::sync::Mutex;
use std::thread::{spawn, yield_now};

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
    /// The "new" function creates a new "Networker" with the given settings
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

        let mut raw_host: [i8; 12] = [0i8; 12];
        let b = host.as_bytes();
        if b.len() != 12 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid host, should be 12 bytes.",
            ));
        } else {
            unsafe {
                // zero-cost cast u8 -> i8
                ptr::copy_nonoverlapping(b.as_ptr() as *const i8, raw_host.as_mut_ptr(), b.len());
            }
        }

        let result = unsafe {
            start(
                &mut raw_net,
                NetworkerSettings {
                    host: raw_host,
                    port,
                    max_queue,
                    max_clients,
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
        return Err(Error::from_raw_os_error(result));
    }

    /// The networker runs in cycles, moving to the next one require this function being called
    ///
    /// I suggest you to have a loop running this during the entire program if you need full uptime
    ///
    /// # Panic
    /// It panics if you forget to initialize your networker
    pub fn cycle(&mut self) {
        if self.initilized != 1 {
            panic!("You forgot to initialize your networker :)")
        }
        unsafe { cycle(&mut self.primitive_self as *mut RawNetworker) };
    }

    /// Return a client struct so you can run operations
    ///
    /// # Panic
    /// Panics if you forget to initialize your networker
    pub fn get_client(&mut self) -> Option<ClientHandler> {
        if self.initilized != 1 {
            panic!("You forgot to initialize your networker :)")
        }
        let a = unsafe { get_client(&mut self.primitive_self) };
        if a.exists > 0 {
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

/*
````````````````````````````````````````````````````````````````````````````````````````````````````
````````````````````````````````````````````````````````````````````````````````````````````````````
````````````````````````````````````````````````````````````````````````````````````````````````````
``````````````````````````````````````````````````‹`````````````````````````````````````````````````
`````````````````````````````````````````````¯36666663¯`````````````````````````````````````````````
``````````````````````````````````````````‡66666666666666‡``````````````````````````````````````````
```````````````````````````````````````‡66666666666666666666‡```````````````````````````````````````
````````````````````````````````````‡66666666666666666666666666‡````````````````````````````````````
````````````````````````````````*ü66666666666666666666666666666666ü*````````````````````````````````
`````````````````````````````l6666666666666666666666666666666666666666l`````````````````````````````
``````````````````````````‡6666666666666666666666666666666666666666666666l``````````````````````````
``````````````````````‹36666666666666666666ü‡*‹``````‹*‡ü66666666666666666663```````````````````````
```````````````````*ü66666666666666666ü‹````````````````````‹ü66666666666666666ü*```````````````````
````````````````l66666666666666666ül````````````````````````````lü66666666666666666*````````````````
`````````````l666666666666666666l``````````````````````````````````l666666666666666666l`````````````
```````````3666666666666666666¯``````````````````````````````````````¯6666666666666666ÇGÇ```````````
``````````¯66666666666666666‡``````````````````````````````````````````l66666666666ÇGggggl``````````
``````````¯666666666666666ü``````````````````````````````````````````````3666666ÇGgggggggl``````````
``````````¯66666666666666l````````````````````````````````````````````````l6ÇÞGggggggggggl``````````
``````````¯6666666666666l````````````````````````‹‹```````````````````````*Ggggggggggggggl``````````
``````````¯666666666666l```````````````````‡666666666666‡``````````````¯gggggggggggggggggl``````````
``````````¯66666666666‡`````````````````‹66666666666666666ü‹````````*Þgggggggggggggggggggl``````````
``````````¯6666666666ü`````````````````ü66666666666666666666ü````‡Gggggggggggggggggggggggl``````````
``````````¯6666666666*```````````````‹66666666666666666666666ÇÞGgggggggggggggggggggggggggl``````````
``````````¯6666666666````````````````666666666666666666666Þggggggggggggggggggggggggggggggl``````````
``````````¯666666666ü```````````````666666666666666666ÇÞgggggggggggggggggggggggggggggggggl``````````
``````````¯6666666663```````````````666666666666666ÇGggggggggggggggggggggggggggggggggggggl``````````
``````````¯6666666663``````````````¯666666666666Çggggggggggggggggggggggggggggggggggggggggl``````````
``````````¯6666666663```````````````66666666ÇÞgÅÅÅÅÅgggggggggggggggggggggggggggggggggggggl``````````
``````````¯666666666ü```````````````66666ÇGgÅÅÅÅÅÅÅÅÅÅÅÅgggggggggggggggggggggggggggggggggl``````````
``````````¯666666666ü````````````````6ÇgÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅggggggggggggggggggggggggggggggl``````````
``````````¯6666666666*```````````````‡ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅGGgggggggggggggggggggggggggl``````````
``````````¯6666666666ü````````````````¯ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ¯```¯6ggggggggggggggggggggggl``````````
``````````¯66666666666‡`````````````````‡ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ‡`````````‡Gggggggggggggggggggl``````````
``````````¯666666666666l```````````````````ÞÅÅÅÅÅÅÅÅÅÅÅÅÞ```````````````lggggggggggggggggl``````````
``````````¯66666666666ÇGü``````````````````````‹l‡3l‹``````````````````````Þgggggggggggggl``````````
``````````¯66666666ÇGÅÅÅÅ6````````````````````````````````````````````````6ÅÅÅÅggggggggggl``````````
``````````¯66666ÇgÅÅÅÅÅÅÅÅg``````````````````````````````````````````````gÅÅÅÅÅÅÅÅgggggggl``````````
``````````¯66ÞÅÅÅÅÅÅÅÅÅÅÅÅÅÅÇ``````````````````````````````````````````ÇÅÅÅÅÅÅÅÅÅÅÅÅÅggggl``````````
```````````ÞÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅl``````````````````````````````````````lÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÞ```````````
`````````````6ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅü``````````````````````````````````üÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ6`````````````
````````````````3ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅü````````````````````````````ügÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ3````````````````
```````````````````‡ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅgl````````````````````*gÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ3```````````````````
``````````````````````*ÞÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÞ3l¯````¯l3ÞÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÞ*``````````````````````
``````````````````````````6ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ6``````````````````````````
`````````````````````````````üÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅü`````````````````````````````
````````````````````````````````3ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅ3````````````````````````````````
```````````````````````````````````*ÞÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÞ*```````````````````````````````````
```````````````````````````````````````ÇÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÅÞ```````````````````````````````````````
``````````````````````````````````````````6ÅÅÅÅÅÅÅÅÅÅÅÅÅÅÇ``````````````````````````````````````````
`````````````````````````````````````````````3gÅÅÅÅÅÅg3`````````````````````````````````````````````
````````````````````````````````````````````````‹ll‹````````````````````````````````````````````````
````````````````````````````````````````````````````````````````````````````````````````````````````
````````````````````````````````````````````````````````````````````````````````````````````````````
````````````````````````````````````````````````````````````````````````````````````````````````````
*/

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

// Message headers
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MessageHeaders {
    pub size: u64,
    pub compr_alg: u8,
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

pub struct ClientHandler {
    inner: *mut Client,
    owner: *mut RawNetworker,
    mutex: *mut Mutex<()>,
}
impl Drop for ClientHandler {
    fn drop(&mut self) {
        let _a = unsafe { *self.mutex.read().lock().unwrap() };
        unsafe { kill_client(self.owner, (*self.inner).id) };
    }
}

impl ClientHandler {
    pub fn get_request(&self) -> (CompressionAlgorithm, Vec<u8>) {
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
        unsafe { ((*self.inner).req_headers.compr_alg.into(), vec) }
    }
}

// Compression algorithms
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum CompressionAlgorithm {
    None = 0,
    LZMA = 1,
    GZIP = 2,
    LZ4 = 3,
    ZSTD = 4,
}

impl From<u8> for CompressionAlgorithm {
    fn from(value: u8) -> Self {
        match value {
            0 => CompressionAlgorithm::None,
            1 => CompressionAlgorithm::LZMA,
            2 => CompressionAlgorithm::GZIP,
            3 => CompressionAlgorithm::LZ4,
            4 => CompressionAlgorithm::ZSTD,
            _ => CompressionAlgorithm::None, // fallback
        }
    }
}

impl From<CompressionAlgorithm> for u8 {
    fn from(value: CompressionAlgorithm) -> Self {
        value as u8
    }
}
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
    pub host: [c_char; 12],
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
    pub ring: *mut [u8; 0], // opaque io_uring struct, we don't manipulate it in Rust
    pub author_log: *mut u64,
}

// Rust helper struct
#[repr(C)]
pub struct SomeClient {
    client: *mut Client,
    pub exists: usize,
}

// Extern functions
unsafe extern "C" {
    fn start(self_: *mut RawNetworker, settings: NetworkerSettings) -> c_int;
    fn proc(self_: *mut RawNetworker) -> c_int;
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
