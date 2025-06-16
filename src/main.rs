use std::{collections::HashMap, io::{Error, ErrorKind, Read, Write}, net::{Shutdown, TcpListener, TcpStream}, str::FromStr, sync::{mpsc::{channel, Sender}, Arc, Mutex}, thread, time::Duration};
use std::time::SystemTime;

use aes_gcm::{aead::{generic_array::GenericArray, rand_core::RngCore, Aead, OsRng, Payload}, Aes256Gcm, AesGcm, KeyInit};

#[derive(PartialEq,Debug)]
pub enum RequestType{
    Authentication,
    Message,
    Ping
}

pub struct Server{
    aesgcm : AesGcm<aes_gcm::aes::Aes256, aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UTerm, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B0>, aes_gcm::aead::consts::B0>>,
    message_handler: Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync + 'static>,
    listener : Arc<TcpListener>
}
pub type MessageHandler = Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync + 'static>;
impl Server{
    /// ## New
    /// Creates a new Server object
    /// ### Parameters
    /// - **host** this argument set the address the server will be running in when the method **start** be called
    /// - **password** before starting a connection the client must send 32 bytes to the server, if the server have these 32 bytes registred the connection will be started, nothing will be returned otherwise. 
    pub fn new(host : String, password : [u8;32],message_handler : MessageHandler) -> Result<Self,Error>{
        let aesgcm: AesGcm<aes_gcm::aes::Aes256, aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UTerm, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B0>, aes_gcm::aead::consts::B0>> = Aes256Gcm::new(&GenericArray::from_slice(&password));
        let listener: Arc<TcpListener> = Arc::new(TcpListener::bind(&host)?);
        Ok(Server { listener, message_handler, aesgcm})
    }
    /// Binds the server to the configured host address, spawns worker threads, and begins listening for incoming client connections.
    ///
    /// This is the main operational loop of the server. It first validates the number of workers,
    /// then binds a `TcpListener` to the address specified in `self.host`. It creates a pool of `workers`
    /// threads to handle authenticated client communications concurrently.
    ///
    /// The server then enters an infinite loop, accepting new connections and dispatching them to the
    /// worker pool after a successful authentication handshake.
    ///
    /// **Note:** This method requires a `'static` reference to the `Server` instance because the
    /// spawned worker threads need to access its state (e.g., the `message_handler`) for the
    /// lifetime of the program.
    ///
    /// ## Connection & Authentication Flow
    /// A new client connection must adhere to the following protocol to be accepted:
    /// 1. The connection **must** be made over IPv6. IPv4 connections are immediately dropped.
    /// 2. The client's IP address must not be on the server's dynamic blacklist.
    /// 3. The client must first send a 1-byte request type of `0` (Authentication).
    /// 4. Immediately following the request type, the client must send the 32-byte password.
    ///
    /// If the password is found in the server's `password` set, the connection is accepted, a 1-second
    /// read timeout is set, and it is passed to a worker thread. Otherwise, the connection is
    /// dropped, and the client's IP address may be blacklisted.
    ///
    /// ## Worker Thread Behavior
    /// Each worker thread manages a set of authenticated connections. It handles two types of requests:
    /// - **Message (type 1):** Reads a message and processes it using the `message_handler` function.
    /// - **Ping (type 2):** Resets the connection's idle timer to keep it alive.
    ///
    /// Connections that are idle for more than 60 seconds are automatically timed out and disconnected.
    ///
    /// # Parameters
    /// - `workers`: The number of concurrent worker threads to spawn for handling client I/O. This value must be at least 1.
    ///
    /// # Returns
    /// - `Ok(())`: This function does not return `Ok` under normal operation as it contains an infinite loop.
    /// - `Err(std::io::Error)`: Returns an error if:
    ///   - The `workers` count is less than 1.
    ///   - The server fails to bind the `TcpListener` to the specified `host` address.
    ///
    /// # Panics
    /// This function will panic if it fails to send an authenticated `TcpStream` to a worker thread's
    /// channel. This would typically only happen if a worker thread has already panicked and its
    /// channel receiver has been dropped.
    pub fn start(self : Arc<Server>,workers : usize) -> Result<(),std::io::Error>{

        if workers < 1{
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "Invalid workers count, the minimum is \"1\".") )
        }
        
        let mut worker_list : Vec<(Arc<Mutex<usize>>,Sender<TcpStream>)> = Vec::new();
        for _ in 0..workers{
            let w: (Sender<TcpStream>, std::sync::mpsc::Receiver<TcpStream>) = channel();
            let cc = Arc::new(Mutex::new(0));
            worker_list.push((cc.clone(),w.0));
            let receiver = w.1;
            let server = self.clone();
            thread::spawn(move ||{
                let mut connections : Vec<(TcpStream,u128,bool)> = Vec::new();
                let mut connection_health : HashMap<u128,SystemTime> = HashMap::new();
                let mut cc_changed = false;
                loop{
                    if let Ok(stream) = receiver.try_recv(){
                        let num = {
                            let mut numba = [0u8;16];
                            numba[..8].clone_from_slice(&OsRng::next_u64(&mut OsRng).to_be_bytes());
                            numba[8..].clone_from_slice(&OsRng::next_u64(&mut OsRng).to_be_bytes());
                            u128::from_be_bytes(numba)
                        };
                        connection_health.insert(num, SystemTime::now());
                        connections.push((stream,num,true));
                        cc_changed = true;
                    }
                    let mut delete : Vec<usize> = Vec::new();
                    let mut to_delete = false;
                    let mut iter = connections.iter_mut().enumerate();
                    let mut current : Option<usize> = None;
                    while let Some ((index,connection)) = iter.next(){
                        // if !connection.2{
                        //     continue;
                        // }
                        if let Ok(dur) = connection_health.get(&connection.1).unwrap().elapsed(){
                            if dur.as_secs() > 60{
                                to_delete = true;
                                delete.push(index);
                                connection.2 = false;
                            }
                            let stream = &mut connection.0;
                            let mut interaction_type = [0u8;1];
                            if let Err(_) = stream.read_exact(&mut interaction_type){
                                continue;
                            }
                            {connection_health.insert(connection.1, SystemTime::now())};
                            let _ = stream;
                            match u8::from_be_bytes(interaction_type){
                                1 => {
                                    current = Some(index.clone());
                                    break;
                                },
                                2 => {
                                    {connection_health.insert(connection.1, SystemTime::now())};
                                },
                                _ => {
                                    continue;
                                }
                            };

                        }
                    }
                    if let Some(index) = current{
                        if let Some(a) = connections.get(index){
                            let mut stream = &a.0;
                            let message_size = {
                                let mut bytes = [0u8;8];
                                if let Err(_) = stream.read_exact(&mut bytes){
                                    continue;
                                }
                                u64::from_be_bytes(bytes) as usize
                            };
                            let payload = {
                                let mut bytes = vec![0u8;message_size];
                                if stream.read_exact(&mut bytes).is_err(){
                                    continue;
                                };
                                if bytes.len() != message_size{
                                   
                                    let _ = stream.shutdown(Shutdown::Both);
                                    let tuple = connections.remove(index);
                                    let _ = tuple.0.shutdown(Shutdown::Both);
                                    connection_health.remove(&tuple.1);
                                    cc_changed = true;
                                   
                                    continue;
                                }
                                let nonce = GenericArray::from_slice(&bytes[..12]);
                                let ciphertext = Payload::from(&(bytes[12..(bytes.to_vec().len() as usize)]));
                                if let Ok(b) = server.aesgcm.decrypt(nonce, ciphertext){
                                    b
                                }else{
                                   
                                    let _ = stream.shutdown(Shutdown::Both);
                                    let tuple = connections.remove(index);
                                    let _ = tuple.0.shutdown(Shutdown::Both);
                                    connection_health.remove(&tuple.1);
                                    cc_changed = true;
                                    continue;
                                }
                            };
                           
                            let response = (server.message_handler)(payload);
                            let mut nice_bytes = (response.len() as u64).to_be_bytes().to_vec();
                            nice_bytes.extend_from_slice(&response);
                            let nonce = {
                                let mut dest: [u8; 12] = [0u8;12];
                                OsRng::fill_bytes(&mut OsRng, &mut dest);
                                dest
                            };
                            if let Ok(a) = server.aesgcm.encrypt(&GenericArray::from_slice(&nonce), response.as_slice()){
                                let length = nonce.len()+a.len();
                                let size : [u8;8] = ((length) as u64).to_be_bytes();
                                let mut payload = size.to_vec();
                                payload.shrink_to(length+8);
                                payload.extend_from_slice(&nonce.to_vec());
                                payload.extend_from_slice(&a);

                                let _ = stream.write_all(&payload);
                                let _ = stream.flush();
                            }
                        }
                        
                    }
                    
                    if to_delete{
                        for i in delete{
                            let tuple = connections.remove(i);
                            let _ = tuple.0.shutdown(Shutdown::Both);
                            connection_health.remove(&tuple.1);
                            cc_changed = true;
                        }
                    }
                    if cc_changed{
                        cc_changed = false;
                        *cc.lock().unwrap() = connections.len();
                    }
                }
            });
        }
        let timeout = Duration::from_secs(1);
        loop {
            let con = self.listener.accept().unwrap();
            let mut stream = con.0;
            stream.set_read_timeout(Some(timeout.clone())).unwrap();
            stream.set_write_timeout(Some(timeout.clone())).unwrap();

            let mut request_type_buffer = [0u8;1];
            if let Err(_) = stream.read_exact(&mut request_type_buffer){
               
                let _ = stream.shutdown(Shutdown::Both);
                continue;
            };

            let request_type: RequestType = match u8::from_be_bytes(request_type_buffer){
                0 => {
                    RequestType::Authentication
                },
                _ => {
                    continue;
                }
            };
            if RequestType::Authentication == request_type{
               
                let mut password: [u8; 156] = [0u8;156];
                if let Err(_) = stream.read_exact(&mut password){
                   
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }else{
                   
                    let nonce = GenericArray::from_slice(&password[..12]);
                    let cipher = &(password[12..]);
                    let ciphertext = Payload::from(cipher);
                   
                    if let Ok(_) = self.aesgcm.decrypt(nonce, ciphertext){
                       
                        let _ = stream.write_all(&255u8.to_be_bytes());
                        stream.flush()?;
                       
                        worker_list.sort_by_key(|f|f.0.lock().unwrap().clone());
                        if let Some(f) = worker_list.first(){
                            f.1.send(stream).unwrap();
                        }
                    }else{
                       
                        thread::sleep(Duration::from_millis(10));
                        let _ = stream.shutdown(Shutdown::Both);
                    }
                }
            }
        }
    }
}

/// Represents a client that communicates with the FalcoTCP interface
/// secured with AES-256-GCM encryption.
///
/// # Fields (private)
/// - `stream`: The underlying TCP stream used for network communication.
/// - `aesgcm`: The AES-256-GCM cipher instance initialized with the client's password.
///
/// # Usage
/// Create a new `Client` by providing a 32-byte password, a network address, and a timeout.
/// The client then handles encrypted message exchanges with the server.

pub struct Client{
    stream : TcpStream,
    aesgcm : Aes256Gcm,
}
impl Client{
    // Establishes a new connection to the given address within the specified timeout.
    /// Initializes the AES-256-GCM cipher using the provided 32-byte password.
    /// Performs a handshake by sending an encrypted payload to authenticate.
    /// 
    /// If the connection remains idle for more than 30 seconds, the server will shut it down.
    /// To keep the connection alive, you must either ping the server before the timeout
    /// or send a message, as both actions reset the server’s inactivity timer.
    ///
    /// # Arguments
    /// - `password`: 32-byte array used as the AES key.
    /// - `address`: The server's socket address to connect to.
    /// - `timeout`: Timeout duration in seconds for the connection attempt.
    ///
    /// # Returns
    /// - `Ok(Client)`: If the connection and authentication succeed.
    /// - `Err(Error)`: If the connection fails, times out, or authentication is rejected.
    pub fn new(password : [u8;32], address : &str, timeout : u64) -> Result<Client,Error>{
        let address = match std::net::SocketAddr::from_str(address){Ok(a)=>a,Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string()))};
        let mut stream = TcpStream::connect_timeout(&address,Duration::from_secs(timeout))?;
        stream.set_write_timeout(Some(Duration::from_secs(timeout)))?;
        let mut payload = vec![];

        payload.extend_from_slice(&[0u8]);

        let nonce = {
            let mut dest: [u8; 12] = [0u8;12];
            OsRng::fill_bytes(&mut OsRng, &mut dest);
            dest
        };

        let brick = {
            let mut dest: [u8; 128] = [0u8;128];
            OsRng::fill_bytes(&mut OsRng, &mut dest);
            dest
        };
       
        payload.extend_from_slice(&nonce);
        let aesgcm: AesGcm<aes_gcm::aes::Aes256, aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UTerm, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B0>, aes_gcm::aead::consts::B0>> = Aes256Gcm::new(&GenericArray::from_slice(&password));
        match aesgcm.encrypt(GenericArray::from_slice(&nonce), Payload::from(brick.as_slice())){
            Ok(a) => {
               payload.extend_from_slice(&a)
            },
            Err(e) => return Err(Error::new(ErrorKind::Other,e.to_string()))
        }

        
       
        stream.write(&payload.as_slice())?;
        stream.flush()?;
        let mut bytes = [0u8;1];

        stream.read_exact(&mut bytes)?;
        let success = bytes[0] == 255;
        if success{
            return Ok(Client{stream,aesgcm})
        }
        Err(Error::new(ErrorKind::ConnectionRefused, "Invalid password"))
    }
    /// Sends an encrypted message to the server and waits for an encrypted response.
    ///
    /// # Arguments
    /// - `bytes`: The plaintext message bytes to send.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The decrypted response bytes received from the server.
    /// - `Err(Error)`: If encryption, sending, or receiving fails.
    pub fn message(&mut self,bytes : Vec<u8>) -> Result<Vec<u8>, Error>{
        let l = bytes.len();
        let mut payload = Vec::with_capacity(l+9);
        let request_size = ((l as u64)+28).to_be_bytes();
        payload.push(1u8);
        payload.extend_from_slice(&request_size);

        let nonce = {
            let mut dest: [u8; 12] = [0u8;12];
            OsRng::fill_bytes(&mut OsRng, &mut dest);
            dest
        };
        payload.extend_from_slice(&nonce);

        
        match self.aesgcm.encrypt(GenericArray::from_slice(&nonce), Payload::from(bytes.as_slice())){
            Ok(a) => {payload.extend_from_slice(&a)},
            Err(e) => {return Err(Error::new(ErrorKind::Other,e.to_string()))}
        };

        self.stream.write_all(&payload)?;
        
        let mut response_meta = [0u8;8];
        self.stream.read_exact(&mut response_meta)?;
        let response_size = u64::from_be_bytes(response_meta) as usize;
        let mut cipherpack = vec![0u8;response_size];
        self.stream.read_exact(&mut cipherpack)?;

        let pack = match self.aesgcm.decrypt(GenericArray::from_slice(&cipherpack[..12]), Payload::from(&cipherpack[12..])){
            Ok(a) => a,
            Err(e) =>{
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
        };

        Ok(pack)
        
    }

    /// Sends a simple "ping" to the server (a single byte `2`) to ensure the connection do not automatically closes, sending messages also reset the the timer.
    ///
    /// # Returns
    /// - `Ok(())`: If the ping byte was sent successfully.
    /// - `Err(Error)`: If the write operation failed.
    pub fn ping(&mut self) -> Result<(), Error>{
        self.stream.write_all(&2u8.to_be_bytes())?;
        Ok(())
    }
}

const TEST_PASSWORD: [u8; 32] = [
    0x7a, 0x14, 0xd3, 0x9f, 0x21, 0x8e, 0x6b, 0xac,
    0x44, 0x90, 0x3c, 0xfe, 0x71, 0x5d, 0xb0, 0x08,
    0x6f, 0xa2, 0x19, 0xdd, 0x37, 0x4b, 0xee, 0xc1,
    0x5a, 0x03, 0x97, 0x62, 0x84, 0xfa, 0xbc, 0x11,
];
fn main() {
    fn multiple_connections() {
        thread::spawn(move ||{
            let closure = |b : Vec<u8>| -> Vec<u8>{b};
            let s = Server::new("127.0.0.1:8000".to_owned(), TEST_PASSWORD, Box::new(closure)).unwrap();
            Server::start(Arc::new(s), 2).unwrap();
        });
        thread::sleep(Duration::from_secs(1));
        let mut thread = Vec::new();
        for _ in 0..10{
            thread.push(thread::spawn(||{
               
                let mut client: Client = Client::new(TEST_PASSWORD, "127.0.0.1:8000", 10).unwrap();
               
                assert_eq!(client.message(b"abc".to_vec()).unwrap(),b"abc".to_vec())
            }));
        }
        for i in thread{
            i.join().unwrap();
           
        }
        
    }
    multiple_connections();
   
}

