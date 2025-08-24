use std::pin::Pin;
use std::time::SystemTime;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use aes_gcm::{
    Aes256Gcm, AesGcm, KeyInit,
    aead::{Aead, OsRng, Payload, generic_array::GenericArray, rand_core::RngCore},
};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::yield_now;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{
        Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    time::timeout,
};

#[derive(PartialEq, Debug)]
pub enum RequestType {
    Authentication,
    Message,
    Ping,
}

pub struct Server {
    aesgcm: AesGcm<
        aes_gcm::aes::Aes256,
        aes_gcm::aes::cipher::typenum::UInt<
            aes_gcm::aes::cipher::typenum::UInt<
                aes_gcm::aes::cipher::typenum::UInt<
                    aes_gcm::aes::cipher::typenum::UInt<
                        aes_gcm::aes::cipher::typenum::UTerm,
                        aes_gcm::aead::consts::B1,
                    >,
                    aes_gcm::aead::consts::B1,
                >,
                aes_gcm::aead::consts::B0,
            >,
            aes_gcm::aead::consts::B0,
        >,
    >,
    message_handler: MessageHandler,
    listener: Arc<TcpListener>,
}
pub type MessageHandler =
    Arc<dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Vec<u8>> + Send>> + Send + Sync + 'static>;

impl Server {
    pub async fn new(
        host: String,
        password: [u8; 32],
        message_handler: MessageHandler,
        workers: usize,
    ) -> Result<(), Error> {
        let aesgcm: AesGcm<
            aes_gcm::aes::Aes256,
            aes_gcm::aes::cipher::typenum::UInt<
                aes_gcm::aes::cipher::typenum::UInt<
                    aes_gcm::aes::cipher::typenum::UInt<
                        aes_gcm::aes::cipher::typenum::UInt<
                            aes_gcm::aes::cipher::typenum::UTerm,
                            aes_gcm::aead::consts::B1,
                        >,
                        aes_gcm::aead::consts::B1,
                    >,
                    aes_gcm::aead::consts::B0,
                >,
                aes_gcm::aead::consts::B0,
            >,
        > = Aes256Gcm::new(&GenericArray::from_slice(&password));
        let listener: Arc<TcpListener> = Arc::new(TcpListener::bind(&host).await?);
        let server = Server {
            listener,
            message_handler,
            aesgcm,
        };
        Server::start(Arc::new(server), workers).await
    }

    pub async fn start(self: Arc<Server>, workers: usize) -> Result<(), std::io::Error> {
        if workers < 1 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Invalid workers count, the minimum is \"1\".",
            ));
        }

        let mut worker_list: Vec<(Arc<Mutex<usize>>, Sender<TcpStream>)> = Vec::new();
        for _ in 0..workers {
            let w: (Sender<TcpStream>, Receiver<TcpStream>) = channel(10); // Increased buffer
            let cc = Arc::new(Mutex::new(0));
            worker_list.push((cc.clone(), w.0));
            let mut receiver = w.1;
            let server = self.clone();
            tokio::task::spawn(async move {
                let mut connections: Vec<(TcpStream, u128, bool)> = Vec::new();
                let mut connection_health: HashMap<u128, SystemTime> = HashMap::new();
                let mut cc_changed = false;

                loop {
                    if let Ok(stream) = receiver.try_recv() {
                        let num = {
                            let mut numba = [0u8; 16];
                            numba[..8].clone_from_slice(&OsRng::next_u64(&mut OsRng).to_be_bytes());
                            numba[8..].clone_from_slice(&OsRng::next_u64(&mut OsRng).to_be_bytes());
                            u128::from_be_bytes(numba)
                        };
                        connection_health.insert(num, SystemTime::now());
                        connections.push((stream, num, true));
                        cc_changed = true;
                    }

                    let mut delete: Vec<usize> = Vec::new();
                    let mut current: Option<usize> = None;

                    for (index, connection) in connections.iter_mut().enumerate() {
                        if !connection.2 {
                            continue;
                        }

                        if let Ok(dur) = connection_health.get(&connection.1).unwrap().elapsed() {
                            if dur.as_secs() > 60 {
                                delete.push(index);
                                connection.2 = false;
                                continue;
                            }
                        }

                        let stream = &mut connection.0;

                        let interaction_type = if let Ok(result) =
                            timeout(Duration::from_millis(10), async {
                                let mut buffer = [0u8; 1];
                                stream.read_exact(&mut buffer).await.map(|_| buffer[0])
                            })
                            .await
                        {
                            match result {
                                Ok(byte) => byte,
                                Err(_) => {
                                    delete.push(index);
                                    connection.2 = false;
                                    continue;
                                }
                            }
                        } else {
                            continue; // Timeout, try next connection
                        };

                        connection_health.insert(connection.1, SystemTime::now());

                        match interaction_type {
                            1 => {
                                // Message
                                current = Some(index);
                                break;
                            }
                            2 => {
                                // Ping
                                connection_health.insert(connection.1, SystemTime::now());
                            }
                            _ => {
                                continue;
                            }
                        }
                    }

                    if let Some(index) = current {
                        if let Some(connection) = connections.get_mut(index) {
                            let stream = &mut connection.0;

                            let message_size = match timeout(Duration::from_secs(5), async {
                                let mut bytes = [0u8; 8];
                                stream
                                    .read_exact(&mut bytes)
                                    .await
                                    .map(|_| u64::from_be_bytes(bytes) as usize)
                            })
                            .await
                            {
                                Ok(Ok(size)) => size,
                                _ => {
                                    delete.push(index);
                                    connection.2 = false;
                                    continue;
                                }
                            };

                            let payload = match timeout(Duration::from_secs(5), async {
                                let mut bytes = vec![0u8; message_size];
                                stream.read_exact(&mut bytes).await.map(|_| bytes)
                            })
                            .await
                            {
                                Ok(Ok(bytes)) => {
                                    if bytes.len() < 12 {
                                        delete.push(index);
                                        connection.2 = false;
                                        continue;
                                    }

                                    let nonce = GenericArray::from_slice(&bytes[..12]);
                                    let ciphertext = Payload::from(&bytes[12..]);
                                    match server.aesgcm.decrypt(nonce, ciphertext) {
                                        Ok(decrypted) => decrypted,
                                        Err(_) => {
                                            delete.push(index);
                                            connection.2 = false;
                                            continue;
                                        }
                                    }
                                }
                                _ => {
                                    delete.push(index);
                                    connection.2 = false;
                                    continue;
                                }
                            };

                            let response = (server.message_handler)(payload).await;
                            let nonce = {
                                let mut dest: [u8; 12] = [0u8; 12];
                                OsRng::fill_bytes(&mut OsRng, &mut dest);
                                dest
                            };

                            if let Ok(encrypted) = server
                                .aesgcm
                                .encrypt(&GenericArray::from_slice(&nonce), response.as_slice())
                            {
                                let length = nonce.len() + encrypted.len();
                                let size: [u8; 8] = (length as u64).to_be_bytes();

                                let mut response_payload = size.to_vec();
                                response_payload.extend_from_slice(&nonce);
                                response_payload.extend_from_slice(&encrypted);

                                if stream.write_all(&response_payload).await.is_err()
                                    || stream.flush().await.is_err()
                                {
                                    delete.push(index);
                                    connection.2 = false;
                                }
                            }
                        }
                    }

                    if delete.len() > 0 {
                        delete.sort_unstable_by(|a, b| b.cmp(a));
                        for i in delete {
                            if i < connections.len() {
                                let connection = connections.remove(i);
                                connection_health.remove(&connection.1);
                                cc_changed = true;
                            }
                        }
                    }

                    if cc_changed {
                        cc_changed = false;
                        *cc.lock().await = connections.len();
                    }
                    yield_now().await;
                }
            });
        }

        loop {
            let (stream, _) = self.listener.accept().await?;

            let mut stream = stream;

            let mut request_type_buffer = [0u8; 1];
            if let Err(_e) = stream.read_exact(&mut request_type_buffer).await {
                continue;
            };

            let request_type: RequestType = match u8::from_be_bytes(request_type_buffer) {
                0 => RequestType::Authentication,
                _ => {
                    continue;
                }
            };

            if RequestType::Authentication == request_type {
                let mut password: [u8; 156] = [0u8; 156];
                if let Err(_) = stream.read_exact(&mut password).await {
                    continue;
                } else {
                    let nonce = GenericArray::from_slice(&password[..12]);
                    let cipher = &password[12..];
                    let ciphertext = Payload::from(cipher);

                    if let Ok(_) = self.aesgcm.decrypt(nonce, ciphertext) {
                        let _ = stream.write_all(&[255u8]).await;
                        let _ = stream.flush().await;

                        let mut min_connections = usize::MAX;
                        let mut selected_worker = None;

                        for worker in &worker_list {
                            let count = *worker.0.lock().await;
                            if count < min_connections {
                                min_connections = count;
                                selected_worker = Some(&worker.1);
                            }
                        }

                        if let Some(sender) = selected_worker {
                            if let Err(_) = sender.send(stream).await {
                                yield_now().await;
                            }
                        }
                    } else {
                        yield_now().await;
                    }
                }
            }
        }
    }
}

pub struct Client {
    stream: TcpStream,
    aesgcm: Aes256Gcm,
}

impl Client {
    pub async fn new(address: &str, password: [u8; 32]) -> Result<Client, Error> {
        let address = match std::net::SocketAddr::from_str(address) {
            Ok(a) => a,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        let mut stream = TcpStream::connect(&address).await?;
        let mut payload = vec![];

        payload.push(0u8);

        let nonce = {
            let mut dest: [u8; 12] = [0u8; 12];
            OsRng::fill_bytes(&mut OsRng, &mut dest);
            dest
        };

        let brick = {
            let mut dest: [u8; 128] = [0u8; 128];
            OsRng::fill_bytes(&mut OsRng, &mut dest);
            dest
        };

        payload.extend_from_slice(&nonce);
        let aesgcm = Aes256Gcm::new(&GenericArray::from_slice(&password));

        match aesgcm.encrypt(
            GenericArray::from_slice(&nonce),
            Payload::from(brick.as_slice()),
        ) {
            Ok(encrypted) => payload.extend_from_slice(&encrypted),
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        }

        stream.write_all(&payload).await?;
        stream.flush().await?;

        let mut response = [0u8; 1];

        if let Err(_) = timeout(Duration::from_secs(5), stream.read_exact(&mut response)).await {
            return Err(Error::new(ErrorKind::TimedOut, "Authentication timeout"));
        }

        let success = response[0] == 255;
        if success {
            return Ok(Client { stream, aesgcm });
        }

        Err(Error::new(ErrorKind::ConnectionRefused, "Invalid password"))
    }

    pub async fn message(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
        let nonce = {
            let mut dest: [u8; 12] = [0u8; 12];
            OsRng::fill_bytes(&mut OsRng, &mut dest);
            dest
        };

        let encrypted = match self.aesgcm.encrypt(
            GenericArray::from_slice(&nonce),
            Payload::from(bytes.as_slice()),
        ) {
            Ok(enc) => enc,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        let total_size = nonce.len() + encrypted.len();
        let mut payload = Vec::new();
        payload.push(1u8); // Message request type
        payload.extend_from_slice(&(total_size as u64).to_be_bytes());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&encrypted);

        self.stream.write_all(&payload).await?;
        self.stream.flush().await?;

        let mut response_size_bytes = [0u8; 8];
        self.stream.read_exact(&mut response_size_bytes).await?;
        let response_size = u64::from_be_bytes(response_size_bytes) as usize;

        let mut response_payload = vec![0u8; response_size];
        self.stream.read_exact(&mut response_payload).await?;

        if response_payload.len() < 12 {
            return Err(Error::new(ErrorKind::Other, "Response too small"));
        }

        let response_nonce = GenericArray::from_slice(&response_payload[..12]);
        let response_ciphertext = Payload::from(&response_payload[12..]);

        match self.aesgcm.decrypt(response_nonce, response_ciphertext) {
            Ok(decrypted) => Ok(decrypted),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }

    pub async fn ping(&mut self) -> Result<(), Error> {
        self.stream.write_all(&[2u8]).await?;
        self.stream.flush().await?;
        Ok(())
    }
}
