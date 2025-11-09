use std::thread::JoinHandle;

#[test]
fn run() {
    use std::{
        hash::{DefaultHasher, Hasher},
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };

    use crate::falco_pipeline::Var;
    use crate::networker::Networker;
    use crate::{
        enums::CompressionAlgorithm,
        falco_client::FalcoClient,
        falco_pipeline::{pipeline_receive, pipeline_send},
    };
    use log::info;
    use rcgen::generate_simple_self_signed;
    use std::{
        fs,
        sync::{Arc, Mutex},
        thread,
    };

    info!("TESTING: TLS SERVER AND CLIENT // THREAD");

    const WORKERS: usize = 1;

    let subject_alt_names: Vec<String> = vec!["localhost".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names).unwrap();

    fs::write("/tmp/cert.pem", cert.cert.pem()).unwrap();
    fs::write("/tmp/key.pem", cert.signing_key.serialize_pem()).unwrap();

    let networker = Arc::new(Mutex::new(
        Networker::new("127.0.0.1", 6000, 10, 100, "/tmp/cert.pem", "/tmp/key.pem").unwrap(),
    ));

    let networker_work = move |networker_share: Arc<Mutex<Networker>>| {
        let var = Var {
            compression: CompressionAlgorithm::None,
        };
        loop {
            thread::yield_now();
            let mut main = networker_share.lock().unwrap();
            main.cycle();
            let client = main.get_client();
            drop(main);
            if let Some(client) = client {
                let request = client.get_request();
                let result = pipeline_receive(request.0.into(), request.1, &var);
                let res = if let Ok(res) = result {
                    res
                } else {
                    client.kill();
                    continue;
                };
                let mut hasher = DefaultHasher::new();
                hasher.write(&res);
                let h = hasher.finish();
                let payload = h.to_be_bytes().to_vec();
                let sending = pipeline_send(payload, &var);
                if let Ok(load) = sending {
                    client.apply_response(load.1, load.0.into()).unwrap();
                } else {
                    client.kill();
                }
            }
        }
    };

    let client_work = || {
        let data = vec![
            vec![0x00, 0x01, 0x02, 0x03],
            vec![0x04, 0x05, 0x06, 0x07],
            vec![0x08, 0x09, 0x0A, 0x0B],
            vec![0x0C, 0x0D, 0x0E, 0x0F],
            vec![0x10, 0x11, 0x12, 0x13],
            vec![0x14, 0x15, 0x16, 0x17],
            vec![0x18, 0x19, 0x1A, 0x1B],
            vec![0x1C, 0x1D, 0x1E, 0x1F],
            vec![0x20, 0x21, 0x22, 0x23],
            vec![0x24, 0x25, 0x26, 0x27],
            vec![0x28, 0x29, 0x2A, 0x2B],
            vec![0x2C, 0x2D, 0x2E, 0x2F],
            vec![0x30, 0x31, 0x32, 0x33],
            vec![0x34, 0x35, 0x36, 0x37],
            vec![0x38, 0x39, 0x3A, 0x3B],
            vec![0x3C, 0x3D, 0x3E, 0x3F],
            vec![0x40, 0x41, 0x42, 0x43],
            vec![0x44, 0x45, 0x46, 0x47],
            vec![0x48, 0x49, 0x4A, 0x4B],
            vec![0x4C, 0x4D, 0x4E, 0x4F],
            vec![0x50, 0x51, 0x52, 0x53],
            vec![0x54, 0x55, 0x56, 0x57],
            vec![0x58, 0x59, 0x5A, 0x5B],
            vec![0x5C, 0x5D, 0x5E, 0x5F],
            vec![0x60, 0x61, 0x62, 0x63],
            vec![0x64, 0x65, 0x66, 0x67],
            vec![0x68, 0x69, 0x6A, 0x6B],
            vec![0x6C, 0x6D, 0x6E, 0x6F],
            vec![0x70, 0x71, 0x72, 0x73],
            vec![0x74, 0x75, 0x76, 0x77],
            vec![0x78, 0x79, 0x7A, 0x7B],
            vec![0x7C, 0x7D, 0x7E, 0x7F],
            vec![0x80, 0x81, 0x82, 0x83],
            vec![0x84, 0x85, 0x86, 0x87],
            vec![0x88, 0x89, 0x8A, 0x8B],
            vec![0x8C, 0x8D, 0x8E, 0x8F],
            vec![0x90, 0x91, 0x92, 0x93],
            vec![0x94, 0x95, 0x96, 0x97],
            vec![0x98, 0x99, 0x9A, 0x9B],
            vec![0x9C, 0x9D, 0x9E, 0x9F],
            vec![0xA0, 0xA1, 0xA2, 0xA3],
            vec![0xA4, 0xA5, 0xA6, 0xA7],
            vec![0xA8, 0xA9, 0xAA, 0xAB],
            vec![0xAC, 0xAD, 0xAE, 0xAF],
            vec![0xB0, 0xB1, 0xB2, 0xB3],
            vec![0xB4, 0xB5, 0xB6, 0xB7],
            vec![0xB8, 0xB9, 0xBA, 0xBB],
            vec![0xBC, 0xBD, 0xBE, 0xBF],
            vec![0xC0, 0xC1, 0xC2, 0xC3],
            vec![0xC4, 0xC5, 0xC6, 0xC7],
            vec![0xC8, 0xC9, 0xCA, 0xCB],
            vec![0xCC, 0xCD, 0xCE, 0xCF],
            vec![0xD0, 0xD1, 0xD2, 0xD3],
            vec![0xD4, 0xD5, 0xD6, 0xD7],
            vec![0xD8, 0xD9, 0xDA, 0xDB],
            vec![0xDC, 0xDD, 0xDE, 0xDF],
            vec![0xE0, 0xE1, 0xE2, 0xE3],
            vec![0xE4, 0xE5, 0xE6, 0xE7],
            vec![0xE8, 0xE9, 0xEA, 0xEB],
            vec![0xEC, 0xED, 0xEE, 0xEF],
            vec![0xF0, 0xF1, 0xF2, 0xF3],
            vec![0xF4, 0xF5, 0xF6, 0xF7],
            vec![0xF8, 0xF9, 0xFA, 0xFB],
            vec![0xFC, 0xFD, 0xFE, 0xFF],
        ];
        let param = Var {
            compression: CompressionAlgorithm::None,
        };
        let sock = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6000);
        let client = FalcoClient::new(
            1,
            param,
            &sock,
            (
                Duration::from_secs(30),
                Duration::from_secs(30),
                Duration::from_secs(30),
            ),
            true,
            "localhost",
        )
        .unwrap();
        for g in data {
            let mut d = DefaultHasher::new();
            let result = client.request(g.clone()).unwrap();
            d.write(&g);
            let trush = d.finish();
            assert_eq!(result.len(), 8);
            assert_eq!(result, trush.to_be_bytes().to_vec());
        }
    };

    for _ in 0..WORKERS {
        let t = Arc::clone(&networker);
        thread::spawn(move || networker_work(t));
    }

    thread::sleep(Duration::from_secs(1));
    let mut handler = Vec::new();
    for _ in 0..WORKERS {
        handler.push(thread::spawn(client_work));
    }
    for i in handler {
        i.join().unwrap()
    }
}
