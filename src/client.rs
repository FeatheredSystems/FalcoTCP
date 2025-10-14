use crate::networker::{CompressionAlgorithm, MessageHeaders};
#[cfg(not(feature = "tokio-runtime"))]
use std::net::TcpStream;
use std::{
    io::{Error, Read, Write},
    net::SocketAddr,
    time::Duration,
};

pub struct Client {
    socket: TcpStream,
}
pub struct Response {
    pub headers: MessageHeaders,
    pub buffer: Vec<u8>,
}

impl Client {
    ///
    /// - Timeout: The durations represent the time required to timeout, they being for write, read, and connection.
    pub fn new(timeout: (Duration, Duration, Duration), adr: &SocketAddr) -> Result<Self, Error> {
        let socket = TcpStream::connect_timeout(adr, timeout.2)?;
        socket.set_read_timeout(Some(timeout.1))?;
        socket.set_write_timeout(Some(timeout.0))?;
        return Ok(Client { socket });
    }
    pub fn request(
        &mut self,
        input: Vec<u8>,
        alg: CompressionAlgorithm,
    ) -> Result<Response, Error> {
        let headers = MessageHeaders {
            size: input.len() as u64,
            compr_alg: alg.into(),
        };
        let mut buffer = Vec::with_capacity(9 + input.len());
        buffer.extend(&headers.size.to_le_bytes());
        buffer.push(headers.compr_alg as u8);
        buffer.extend_from_slice(&input);
        self.socket.write_all(&buffer)?;

        let mut response_headers = [0u8; 9];
        self.socket.read_exact(&mut response_headers)?;
        let mshead = MessageHeaders {
            size: u64::from_le_bytes({
                let mut a = [0u8; 8];
                a.copy_from_slice(&response_headers[..8]);
                a
            }),
            compr_alg: response_headers[8],
        };
        let mut buffer = vec![0u8; mshead.size as usize];
        if mshead.size > 0 {
            self.socket.read_exact(&mut buffer)?;
        }
        Ok(Response {
            headers: mshead,
            buffer,
        })
    }
}
