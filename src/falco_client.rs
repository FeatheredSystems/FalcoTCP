use crate::falco_pipeline::{pipeline_receive, pipeline_send};
use crate::{client::Client, falco_pipeline::Var};
use std::io::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[cfg(not(feature = "tokio-runtime"))]
use std::sync::{Arc, Mutex, RwLock};

#[cfg(feature = "tokio-runtime")]
use std::sync::Arc;
#[cfg(feature = "tokio-runtime")]
use tokio::{
    sync::{Mutex, RwLock},
    time::timeout,
};

pub struct FalcoClient {
    pub var: Var,
    pub pool: Arc<RwLock<Vec<Arc<Mutex<Client>>>>>,
    target: (SocketAddr, (Duration, Duration, Duration)),
    clock: Instant,
    retry: bool,
}

#[cfg(not(feature = "tokio-runtime"))]
impl FalcoClient {
    pub fn new(
        clients: usize,
        parameters: Var,
        socket: &SocketAddr,
        timeout: (Duration, Duration, Duration),
        retry: bool,
    ) -> Result<Self, Error> {
        let mut v = Vec::with_capacity(clients);
        for _ in 0..clients {
            v.push(Arc::new(Mutex::new(Client::new(timeout.clone(), socket)?)));
        }
        #[cfg(feature = "dev-redundancies")]
        v.shrink_to_fit(); // redundant
        Ok(FalcoClient {
            var: parameters,
            pool: Arc::new(RwLock::new(v)),
            target: (socket.clone(), timeout),
            clock: Instant::now(),
            retry,
        })
    }
    pub fn request(&self, input: Vec<u8>) -> Result<Vec<u8>, Error> {
        let (key, connection) = {
            let pool = self.pool.read().unwrap().clone();
            let len = pool.len();
            let key = self.clock.elapsed().as_nanos() as usize % len;
            let val = pool.get(key);
            if val.is_none() {
                drop(pool);
                return self.mitigate(input, key);
            }
            (key, val.unwrap().clone())
        };
        let mut con = connection.lock().unwrap();
        if !self.retry {
            let woaah = pipeline_send(input, &self.var)?;
            let response = con.request(woaah.1, woaah.0.into())?;
            pipeline_receive(response.headers.compr_alg, response.buffer, &self.var)
        } else {
            let woaah = pipeline_send(input.clone(), &self.var)?;
            let response = match con.request(woaah.1, woaah.0.into()) {
                Ok(a) => a,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::BrokenPipe => return self.mitigate(input, key),
                    _ => return Err(e),
                },
            };

            pipeline_receive(response.headers.compr_alg, response.buffer, &self.var)
        }
    }
    fn mitigate(&self, input: Vec<u8>, key: usize) -> Result<Vec<u8>, Error> {
        self.pool.write().unwrap().swap_remove(key);
        self.generate(1)?;
        self.request(input)
    }
    pub fn generate(&self, count: usize) -> Result<(), Error> {
        let mut pool = self.pool.write().unwrap();
        #[cfg(not(feature = "dev-redundancies"))]
        pool.reserve(count);
        #[cfg(feature = "dev-redundancies")]
        pool.reserve_exact(count);
        for _ in 0..count {
            pool.push(Arc::new(Mutex::new(Client::new(
                self.target.1,
                &self.target.0.clone(),
            )?)));
        }
        Ok(())
    }
}

#[cfg(feature = "tokio-runtime")]
impl FalcoClient {
    pub async fn new(
        clients: usize,
        parameters: Var,
        socket: &SocketAddr,
        timeout: (Duration, Duration, Duration),
        retry: bool,
    ) -> Result<Self, Error> {
        let mut v = Vec::with_capacity(clients);
        for _ in 0..clients {
            v.push(Arc::new(Mutex::new(Client::new(timeout.2, socket).await?)));
        }
        #[cfg(feature = "dev-redundancies")]
        v.shrink_to_fit(); // redundant
        Ok(FalcoClient {
            var: parameters,
            pool: Arc::new(RwLock::new(v)),
            target: (*socket, timeout),
            clock: Instant::now(),
            retry,
        })
    }
    pub async fn request(&self, input: Vec<u8>, prevent_mitigate: bool) -> Result<Vec<u8>, Error> {
        let (key, con) = {
            let pool = self.pool.read().await;
            let len = { pool.len() };
            let key = self.clock.elapsed().as_nanos() as usize % len;
            let val = pool.get(key);
            if val.is_none() {
                drop(pool);
                return Box::pin(self.mitigate(input, key)).await;
            }
            (key, val.unwrap().clone())
        };
        let mut con = con.lock().await;
        if !self.retry {
            let woaah = pipeline_send(input, &self.var)?;
            let response = timeout(self.target.1.0, con.request(woaah.1, woaah.0.into())).await??;
            pipeline_receive(response.headers.compr_alg, response.buffer, &self.var)
        } else {
            let woaah = pipeline_send(input.clone(), &self.var)?;
            let response =
                match timeout(self.target.1.1, con.request(woaah.1, woaah.0.into())).await? {
                    Ok(a) => a,
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::BrokenPipe => {
                            if prevent_mitigate {
                                return Box::pin(self.mitigate(input, key)).await;
                            } else {
                                return Err(e);
                            }
                        }
                        _ => return Err(e),
                    },
                };

            pipeline_receive(response.headers.compr_alg, response.buffer, &self.var)
        }
    }
    async fn mitigate(&self, input: Vec<u8>, key: usize) -> Result<Vec<u8>, Error> {
        self.pool.write().await.swap_remove(key);
        self.generate(1).await?;
        self.request(input, true).await
    }
    pub async fn generate(&self, count: usize) -> Result<(), Error> {
        let mut pool = self.pool.write().await;
        #[cfg(not(feature = "dev-redundancies"))]
        pool.reserve(count);
        #[cfg(feature = "dev-redundancies")]
        pool.reserve_exact(count);
        for _ in 0..count {
            pool.push(Arc::new(Mutex::new(
                timeout(
                    self.target.1.2,
                    Client::new(self.target.1.2, &self.target.0.clone()),
                )
                .await??,
            )));
        }
        Ok(())
    }
}
