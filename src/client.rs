use crate::{CompressionAlgorithm, MessageHeaders};
#[cfg(all(feature = "tls", not(feature = "tokio-tls")))]
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
#[cfg(feature = "tokio-tls")]
use rustls::{ClientConfig, RootCertStore};
#[cfg(any(feature = "tls", feature = "tokio-tls"))]
use rustls_native_certs::load_native_certs;
#[cfg(any(feature = "tls", feature = "tokio-tls"))]
use rustls_pki_types::ServerName;
use std::{io::Error, net::SocketAddr, time::Duration};
#[cfg(not(feature = "tokio-runtime"))]
use std::{
    io::{Read, Write},
    net::TcpStream,
};

#[cfg(feature = "tokio-tls")]
use tokio_rustls::{TlsConnector, client::TlsStream};

#[cfg(feature = "tokio-runtime")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
#[cfg(all(not(feature = "tls"), not(feature = "tokio-tls")))]
pub struct Client {
    socket: TcpStream,
}
#[cfg(all(feature = "tls", not(feature = "tokio-tls")))]
pub struct Client {
    socket: StreamOwned<ClientConnection, TcpStream>,
}
#[cfg(feature = "tokio-tls")]
pub struct Client {
    socket: TlsStream<TcpStream>,
}
pub struct Response {
    pub headers: MessageHeaders,
    pub buffer: Vec<u8>,
}
#[cfg(not(feature = "tokio-runtime"))]
impl Client {
    ///
    /// - Timeout: The durations represent the time required to timeout, they being for write, read, and connection.
    pub fn new(
        timeout: (Duration, Duration, Duration),
        adr: &SocketAddr,
        domain: &str,
    ) -> Result<Self, Error> {
        let con = TcpStream::connect_timeout(adr, timeout.2)?;
        con.set_read_timeout(Some(timeout.1))?;
        con.set_write_timeout(Some(timeout.0))?;

        #[cfg(not(feature = "tls"))]
        {
            return Ok(Client { socket: con });
        }
        #[cfg(feature = "tls")]
        {
            use std::io;
            use std::sync::Arc;
            let mut roots = RootCertStore::empty();
            for cert in load_native_certs().unwrap() {
                roots.add(cert).unwrap();
            }
            let config = Arc::new(
                ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth(),
            );

            let server_name = ServerName::try_from(domain.to_owned())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid server name"))?;
            let conn = ClientConnection::new(config, server_name).map_err(io::Error::other)?;

            let socket = StreamOwned::new(conn, con);
            Ok(Client { socket })
        }
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
        buffer.push(headers.compr_alg);
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
#[cfg(feature = "tokio-runtime")]
impl Client {
    ///
    /// - connection_timeout: Timeouts if the connection take too long to stablish.
    pub async fn new(
        connection_timeout: Duration,
        adr: &SocketAddr,
        domain: &str,
    ) -> Result<Self, Error> {
        let c = timeout(connection_timeout, TcpStream::connect(adr)).await??;
        #[cfg(not(feature = "tokio-tls"))]
        {
            Ok(Client { socket: c })
        }
        #[cfg(feature = "tokio-tls")]
        {
            use std::io;
            use std::sync::Arc;
            let mut roots = RootCertStore::empty();
            for cert in load_native_certs().unwrap() {
                roots.add(cert).unwrap();
            }
            let config = Arc::new(
                ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth(),
            );
            let connector = TlsConnector::from(config);
            let server_name = ServerName::try_from(domain.to_owned())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid server name"))?;
            let socket = connector.connect(server_name, c).await?;
            Ok(Client { socket })
        }
    }
    pub async fn request(
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
        buffer.push(headers.compr_alg);
        buffer.extend_from_slice(&input);
        self.socket.write_all(&buffer).await?;

        let mut response_headers = [0u8; 9];
        self.socket.read_exact(&mut response_headers).await?;
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
            self.socket.read_exact(&mut buffer).await?;
        }
        Ok(Response {
            headers: mshead,
            buffer,
        })
    }
}
