use crate::{client::Client, falco_pipeline::Var};
use std::io::Error;
use std::time::Instant;

#[cfg(not(feature = "tokio-runtime"))]
use std::sync::{Arc, Mutex, RwLock};

#[cfg(feature = "tokio-runtime")]
use std::sync::Arc;
#[cfg(feature = "tokio-runtime")]
use tokio::sync::{Mutex, RwLock};

pub struct FalcoClient {
    pub var: Var,
    pub pool: Arc<RwLock<Vec<Arc<Mutex<Client>>>>>,
    target: (String, u16),
    clock: Instant,
    timeout: usize,
    #[cfg(feature = "tls")]
    domain: String,
    pool_len: usize,
}

impl FalcoClient {
    pub fn new(
        clients: usize,
        parameters: Var,
        host: &str,
        port: u16,
        #[cfg(feature = "tls")] domain: &str,
    ) -> Result<Self, Error> {
        let mut v = Vec::with_capacity(clients);
        for _ in 0..clients {
            v.push(Arc::new(Mutex::new(Client::new(
                host,
                port,
                #[cfg(feature = "tls")]
                domain,
            )?)));
        }
        #[cfg(feature = "dev-redundancies")]
        v.shrink_to_fit(); // redundant
        Ok(FalcoClient {
            var: parameters,
            pool: Arc::new(RwLock::new(v)),
            target: (host.to_string(), port),
            clock: Instant::now(),
            #[cfg(feature = "tls")]
            domain: domain.to_string(),
            timeout: 1_000_000,
            pool_len: clients,
        })
    }
    #[cfg(not(feature = "async"))]
    fn get_handle(&self) -> (Arc<Mutex<Client>>, usize) {
        let index = self.clock.elapsed().as_nanos() as usize % self.pool_len;
        (self.pool.read().unwrap()[index].clone(), index)
    }
    #[cfg(feature = "async")]
    async fn get_handle(&self) -> (Arc<Mutex<Client>>, usize) {
        let index = self.clock.elapsed().as_nanos() as usize % self.pool_len;
        let arc = { self.pool.read().await[index].clone() };
        (arc, index)
    }
    #[cfg(not(feature = "async"))]
    pub fn request(&self, input: Vec<u8>, allow_mitigation: u8) -> Result<Vec<u8>, Error> {
        let (s, k) = self.get_handle();
        match s.lock().unwrap().request(&input, &self.var) {
            Ok(a) => Ok(a),
            Err(e) => {
                use std::io::ErrorKind;

                if e.kind() == ErrorKind::ConnectionAborted && allow_mitigation > 0 {
                    self.mitigate(input, k, allow_mitigation)
                } else {
                    Err(e)
                }
            }
        }
    }
    #[cfg(feature = "async")]
    pub async fn request(&self, input: Vec<u8>, allow_mitigation: u8) -> Result<Vec<u8>, Error> {
        let (s, k) = self.get_handle().await;
        match s.lock().await.request(&input, &self.var).await {
            Ok(a) => Ok(a),
            Err(e) => {
                use std::io::ErrorKind;

                if e.kind() == ErrorKind::ConnectionAborted && allow_mitigation > 0 {
                    Box::pin(self.mitigate(input, k, allow_mitigation)).await
                } else {
                    Err(e)
                }
            }
        }
    }
    #[cfg(not(feature = "async"))]
    fn mitigate(&self, input: Vec<u8>, key: usize, allow_mitigation: u8) -> Result<Vec<u8>, Error> {
        self.pool.write().unwrap().swap_remove(key);
        self.generate(1)?;
        self.request(input, allow_mitigation - 1)
    }
    #[cfg(not(feature = "async"))]
    pub fn generate(&self, count: usize) -> Result<(), Error> {
        let mut pool = self.pool.write().unwrap();
        #[cfg(not(feature = "dev-redundancies"))]
        pool.reserve(count);
        #[cfg(feature = "dev-redundancies")]
        pool.reserve_exact(count);
        for _ in 0..count {
            let mut c = Client::new(
                &self.target.0,
                self.target.1,
                #[cfg(feature = "tls")]
                &self.domain,
            )?;
            c.set_timeout(self.timeout);
            pool.push(Arc::new(Mutex::new(c)));
        }
        Ok(())
    }

    #[cfg(feature = "async")]
    async fn mitigate(
        &self,
        input: Vec<u8>,
        key: usize,
        allow_mitigation: u8,
    ) -> Result<Vec<u8>, Error> {
        self.pool.write().await.swap_remove(key);
        self.generate(1).await?;
        self.request(input, allow_mitigation - 1).await
    }
    #[cfg(feature = "async")]
    pub async fn generate(&self, count: usize) -> Result<(), Error> {
        let mut pool = self.pool.write().await;
        #[cfg(not(feature = "dev-redundancies"))]
        pool.reserve(count);
        #[cfg(feature = "dev-redundancies")]
        pool.reserve_exact(count);
        for _ in 0..count {
            let mut c = Client::new(
                &self.target.0,
                self.target.1,
                #[cfg(feature = "tls")]
                &self.domain,
            )?;
            c.set_timeout(self.timeout);
            pool.push(Arc::new(Mutex::new(c)));
        }
        Ok(())
    }

    #[cfg(not(feature = "tokio"))]
    pub fn set_timeout(&mut self, new_timeout: usize) {
        self.timeout = new_timeout;
        for i in self.pool.read().unwrap().iter() {
            i.lock().unwrap().set_timeout(new_timeout);
        }
    }
    #[cfg(feature = "tokio")]
    pub async fn set_timeout(&mut self, new_timeout: usize) {
        self.timeout = new_timeout;
        for i in self.pool.read().await.iter() {
            i.lock().await.set_timeout(new_timeout);
        }
    }
    pub fn cheap_set_timeout(&mut self, new_timeout: usize) {
        self.timeout = new_timeout;
    }
}
