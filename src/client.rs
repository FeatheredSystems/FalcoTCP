use std::{
    ffi::CString,
    io::{Error, ErrorKind},
    os::raw::c_ushort,
    str::FromStr,
};

/*
typedef struct {
    int fd;
    #if TLS
    SSL* ssl;
    SSL_CTX* ctx;
    #endif
    #if !BLOCKING
    unsigned char *input;
    unsigned char *output;
    MessageHeaders headers[2];
    usize readen;
    usize writen;
    PcAsync processing;
    #endif
} PrimitiveClient;
*/
#[cfg(feature = "async")]
use crate::MessageHeaders;
#[repr(C)]
pub struct Client {
    fd: i32,
    #[cfg(feature = "tls")]
    ssl: usize,
    #[cfg(feature = "tls")]
    ssl_context: usize,
    #[cfg(feature = "async")]
    input: usize,
    #[cfg(feature = "async")]
    output: usize,
    #[cfg(feature = "async")]
    msg_headers: [MessageHeaders; 2],
    #[cfg(feature = "async")]
    readen: usize,
    #[cfg(feature = "async")]
    writen: usize,
    #[cfg(feature = "async")]
    processing: i32,
}

fn zero() -> Client {
    #[cfg(feature = "async")]
    let msg_headers = [
        MessageHeaders {
            compr_alg: 0,
            size: 0,
        },
        MessageHeaders {
            compr_alg: 0,
            size: 0,
        },
    ];
    Client {
        fd: 0,
        ssl: 0,
        ssl_context: 0,
        #[cfg(feature = "async")]
        input: 0,
        #[cfg(feature = "async")]
        output: 0,
        #[cfg(feature = "async")]
        msg_headers,
        #[cfg(feature = "async")]
        readen: 0,
        #[cfg(feature = "async")]
        writen: 0,
        #[cfg(feature = "async")]
        processing: 0,
    }
}

#[repr(C)]
struct Settings {
    host: CString,
    port: c_ushort,
    #[cfg(feature = "tls")]
    domain: CString,
}

/*
*
* typedef struct {
    char* host;
    u_int16_t port;
    #if TLS
        char* domain;
    #endif
} PrimitiveClientSettings;
*/
impl Client {
    pub fn new(
        &self,
        host: &str,
        port: u16,
        #[cfg(feature = "tls")] domain: &str,
    ) -> Result<Self, Error> {
        let mut settings = Settings {
            host: match CString::from_str(host) {
                Ok(a) => a,
                Err(e) => return Err(Error::new(ErrorKind::InvalidInput, e)),
            },
            port,
            #[cfg(feature = "tls")]
            domain: match CString::from_str(domain) {
                Ok(a) => a,
                Err(e) => return Err(Error::new(ErrorKind::InvalidInput, e)),
            },
        };
        let mut client = zero();
        let a = unsafe { pc_create(&mut client, &mut settings) };
        if a >= 0 {
            unsafe { pc_set_timeout(&mut client, 1000000) };
            return Ok(client);
        } else {
            return Err(Error::from_raw_os_error(a));
        }
    }
    pub fn set_timeout(&mut self, micro_secs: usize) {
        unsafe { pc_set_timeout(self, micro_secs) };
    }
}

#[link(name = "raw_client")]
unsafe extern "C" {
    fn pc_create(c: &mut Client, settings: *mut Settings) -> i32;
    #[cfg(not(feature = "async"))]
    fn pc_set_timeout(c: &mut Client, micro_secs: usize);
}
