use crate::falco_pipeline::{pipeline_receive, pipeline_send};
use crate::{client::Client, falco_pipeline::Var};
use std::io::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[cfg(not(feature = "tokio-runtime"))]
use std::sync::{Arc, Mutex, RwLock};

pub struct FalcoClient {
    pub var: Var,
    pub pool: Arc<RwLock<Vec<Arc<Mutex<Client>>>>>,
    target: (SocketAddr, (Duration, Duration, Duration)),
    clock: Instant,
    retry: bool,
}

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
            (key, val.unwrap())
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
        Ok(self.request(input)?)
    }
    pub fn generate(&self, count: usize) -> Result<(), Error> {
        let mut pool = self.pool.write().unwrap();
        #[cfg(not(feature = "dev-redundancies"))]
        pool.reserve(count);
        #[cfg(feature = "dev-redundancies")]
        pool.reserve_exact(count);
        for _ in 0..count {
            pool.push(Arc::new(Mutex::new(Client::new(
                self.target.1.clone(),
                &self.target.0.clone(),
            )?)));
        }
        Ok(())
    }
}
