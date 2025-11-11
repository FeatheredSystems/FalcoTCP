use std::{
    ffi::CString,
    io::{Error, ErrorKind},
    os::raw::c_ushort,
    str::FromStr,
};

use crate::MessageHeaders;
use crate::falco_pipeline::Var;
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

#[repr(i32)]
#[allow(non_camel_case_types)]
#[cfg(feature = "async")]
enum PCASYNC {
    Nothing = 0,
    InputHeaders = 1,
    InputPayload = 2,
    OutputHeaders = 3,
    OutputPayload = 4,
    Done = 5,
}
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
    #[cfg(feature = "async")]
    timeout_time: usize,
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
        #[cfg(feature = "tls")]
        ssl: 0,
        #[cfg(feature = "tls")]
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
        #[cfg(feature = "async")]
        timeout_time: 0,
    }
}

#[repr(C)]
struct Settings {
    host: *mut i8,
    port: c_ushort,
    #[cfg(feature = "tls")]
    domain: *mut i8,
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
    pub fn new(host: &str, port: u16, #[cfg(feature = "tls")] domain: &str) -> Result<Self, Error> {
        let mut settings = Settings {
            host: match CString::from_str(host) {
                Ok(a) => a.into_raw(),
                Err(e) => return Err(Error::new(ErrorKind::InvalidInput, e)),
            },
            port,
            #[cfg(feature = "tls")]
            domain: match CString::from_str(domain) {
                Ok(a) => a.into_raw(),
                Err(e) => return Err(Error::new(ErrorKind::InvalidInput, e)),
            },
        };
        let mut client = zero();
        let a = unsafe { pc_create(&mut client, &mut settings) };
        if a >= 0 {
            unsafe { pc_set_timeout(&mut client, 1000000) };
            Ok(client)
        } else {
            Err(Error::from_raw_os_error(a))
        }
    }
    pub fn set_timeout(&mut self, micro_secs: usize) {
        unsafe { pc_set_timeout(self, micro_secs) };
    }
    #[cfg(not(feature = "async"))]
    pub fn request(&mut self, input: &[u8], var: &Var) -> Result<Vec<u8>, Error> {
        use crate::falco_pipeline::{pipeline_receive, pipeline_send};

        let input = input.to_vec();
        let (compression, mut value) = pipeline_send(input, var)?;
        let input_headers = MessageHeaders {
            compr_alg: compression,
            size: value.len() as u64,
        };
        {
            let res = unsafe { pc_input_request(self, value.as_mut_ptr(), input_headers) };
            if res < 0 {
                return Err(Error::from_raw_os_error(res));
            }
        }
        let buf: *mut u8 = std::ptr::null_mut();
        let mut headers: MessageHeaders = MessageHeaders::default();

        {
            let res = unsafe { pc_output_request(self, &buf, &mut headers) };
            if res < 0 {
                return Err(Error::from_raw_os_error(res));
            }
        }
        let vec = unsafe { Vec::from_raw_parts(buf, headers.size as usize, headers.size as usize) };
        match pipeline_receive(headers.compr_alg, vec, var) {
            Ok(a) => Ok(a),
            Err(e) => Err(e),
        }
    }
    #[cfg(feature = "async")]
    pub async fn request(&mut self, input: &[u8], var: &Var) -> Result<Vec<u8>, Error> {
        let cron = self.timeout_time;
        use tokio::time::timeout;

        use crate::falco_pipeline::{pipeline_receive, pipeline_send};
        use std::time::Duration;

        let input = input.to_vec();
        let (compression, mut value) = match pipeline_send(input, var) {
            Ok(a) => a,
            Err(e) => return Err(e),
        };
        let input_headers = MessageHeaders {
            compr_alg: compression,
            size: value.len() as u64,
        };
        let action = async {
            {
                let res = unsafe { pc_async_input(self, input_headers, value.as_mut_ptr()) };
                if res < 0 {
                    return Err(Error::from_raw_os_error(res));
                }
            }
            while self.processing != PCASYNC::Done as i32 {
                tokio::task::yield_now().await;
                let res = unsafe { pc_async_step(self) };
                if res < 0 {
                    return Err(Error::from_raw_os_error(res));
                }
            }
            let mut output_headers = MessageHeaders::default();
            let buffer: *mut u8 = std::ptr::null_mut();
            let res = unsafe { pc_async_output(self, &mut output_headers, &buffer) };
            if res < 0 {
                return Err(Error::from_raw_os_error(res));
            }
            let output = unsafe {
                Vec::from_raw_parts(
                    buffer,
                    output_headers.size as usize,
                    output_headers.size as usize,
                )
            };
            let response = match pipeline_receive(output_headers.compr_alg, output, var) {
                Ok(a) => a,
                Err(e) => return Err(e),
            };
            return Ok(response);
        };

        match timeout(Duration::from_micros(cron as u64), action).await {
            Ok(result) => result,
            Err(_) => Err(Error::new(ErrorKind::TimedOut, "timeout")),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe { pc_clean(self) };
    }
}

#[link(name = "raw_client")]
unsafe extern "C" {
    fn pc_create(c: &mut Client, settings: *mut Settings) -> i32;
    fn pc_set_timeout(c: &mut Client, micro_secs: usize);
    #[cfg(not(feature = "async"))]
    fn pc_input_request(c: &mut Client, buf: *mut u8, headers: MessageHeaders) -> i32;
    #[cfg(not(feature = "async"))]
    fn pc_output_request(c: &mut Client, buf: &*mut u8, headers: &mut MessageHeaders) -> i32;
    #[cfg(feature = "async")]
    fn pc_async_input(c: &mut Client, headers: MessageHeaders, buffer: *mut u8) -> i32;
    #[cfg(feature = "async")]
    fn pc_async_output(c: &mut Client, headers: &mut MessageHeaders, buffer: &*mut u8) -> i32;
    #[cfg(feature = "async")]
    fn pc_async_step(c: &mut Client) -> i32;
    fn pc_clean(c: &mut Client);
}
