use std::{collections::HashMap, io::{Error, ErrorKind}, net::Shutdown, str::FromStr, sync::Arc, time::Duration};
use std::time::SystemTime;
use std::sync::mpsc::{channel,Receiver, Sender};
use async_std::{
    io::{ReadExt, WriteExt}, net::{TcpListener, TcpStream}, sync::Mutex, task, 
};
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
    pub async fn new(host : String, password : [u8;32],message_handler : MessageHandler) -> Result<Self,Error>{
        let aesgcm: AesGcm<aes_gcm::aes::Aes256, aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UInt<aes_gcm::aes::cipher::typenum::UTerm, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B1>, aes_gcm::aead::consts::B0>, aes_gcm::aead::consts::B0>> = Aes256Gcm::new(&GenericArray::from_slice(&password));
        let listener: Arc<TcpListener> = Arc::new(TcpListener::bind(&host).await?);
        Ok(Server { listener, message_handler, aesgcm})
    }
    
    pub async fn start(self : Arc<Server>,workers : usize) -> Result<(),std::io::Error>{

        if workers < 1{
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "Invalid workers count, the minimum is \"1\".") )
        }
        
        let mut worker_list : Vec<(Arc<Mutex<usize>>,Sender<TcpStream>)> = Vec::new();
        for _ in 0..workers{
            let w: (Sender<TcpStream>, Receiver<TcpStream>) = channel();
            let cc = Arc::new(Mutex::new(0));
            worker_list.push((cc.clone(),w.0));
            let receiver = w.1;
            let server = self.clone();
            task::spawn(async move {
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
                            if let Err(_) = stream.read_exact(&mut interaction_type).await{
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
                                if let Err(_) = stream.read_exact(&mut bytes).await{
                                    continue;
                                }
                                u64::from_be_bytes(bytes) as usize
                            };
                            let payload = {
                                let mut bytes = vec![0u8;message_size];
                                if stream.read_exact(&mut bytes).await.is_err(){
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
                        *cc.lock().await = connections.len();
                    }
                }
            });
        }
        loop {
            let con = self.listener.accept().await.unwrap();
            let mut stream = con.0;

            let mut request_type_buffer = [0u8;1];
            if let Err(_) = stream.read_exact(&mut request_type_buffer).await{
               
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
                if let Err(_) = stream.read_exact(&mut password).await{
                   
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }else{
                   
                    let nonce = GenericArray::from_slice(&password[..12]);
                    let cipher = &(password[12..]);
                    let ciphertext = Payload::from(cipher);
                   
                    if let Ok(_) = self.aesgcm.decrypt(nonce, ciphertext){
                       
                        let _ = stream.write_all(&255u8.to_be_bytes());
                        stream.flush().await?;
                        let mut keyed_workers = Vec::with_capacity(worker_list.len());

                        for worker in &worker_list {
                            let key = worker.0.lock().await.clone();
                            keyed_workers.push((key, worker.clone()));
                        }

                        keyed_workers.sort_by_key(|(key, _)| key.clone());

                        worker_list = keyed_workers.into_iter().map(|(_, worker)| worker).collect();
                        if let Some(f) = worker_list.first(){
                            let _ = f.1.send(stream);
                        }
                    }else{
                       
                        task::sleep(Duration::from_millis(10)).await;
                        let _ = stream.shutdown(Shutdown::Both);
                    }
                }
            }
        }
    }
}



pub struct Client{
    stream : TcpStream,
    aesgcm : Aes256Gcm,
}
impl Client{
   
    pub async fn new(password : [u8;32], address : &str) -> Result<Client,Error>{
        let address = match std::net::SocketAddr::from_str(address){Ok(a)=>a,Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string()))};
        let mut stream = TcpStream::connect(&address).await?;
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

        
       
        stream.write(&payload.as_slice()).await?;
        stream.flush().await?;
        let mut bytes = [0u8;1];

        stream.read_exact(&mut bytes).await?;
        let success = bytes[0] == 255;
        if success{
            return Ok(Client{stream,aesgcm})
        }
        Err(Error::new(ErrorKind::ConnectionRefused, "Invalid password"))
    }
    
    pub async fn message(&mut self,bytes : Vec<u8>) -> Result<Vec<u8>, Error>{
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

        self.stream.write_all(&payload).await?;
        
        let mut response_meta = [0u8;8];
        self.stream.read_exact(&mut response_meta).await?;
        let response_size = u64::from_be_bytes(response_meta) as usize;
        let mut cipherpack = vec![0u8;response_size];
        self.stream.read_exact(&mut cipherpack).await?;

        let pack = match self.aesgcm.decrypt(GenericArray::from_slice(&cipherpack[..12]), Payload::from(&cipherpack[12..])){
            Ok(a) => a,
            Err(e) =>{
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
        };

        Ok(pack)
        
    }

    
    pub async fn ping(&mut self) -> Result<(), Error>{
        self.stream.write_all(&2u8.to_be_bytes()).await?;
        Ok(())
    }
}
