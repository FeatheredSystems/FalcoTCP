#[cfg(feature = "encryption")]
use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, OsRng, rand_core::RngCore},
};
#[cfg(feature = "GZIP")]
use flate2::write::GzEncoder;
#[cfg(feature = "ZSTD")]
use std::ffi::c_void;
use std::io::Error;
#[cfg(feature = "encryption")]
use std::io::ErrorKind;
#[cfg(any(feature = "GZIP", feature = "LZMA"))]
use std::io::Read;
#[cfg(feature = "LZMA")]
use std::io::Write;
#[cfg(feature = "ZSTD")]
use zstd::zstd_safe::zstd_sys::{
    ZSTD_CONTENTSIZE_ERROR, ZSTD_CONTENTSIZE_UNKNOWN, ZSTD_compress, ZSTD_compressBound,
    ZSTD_decompress, ZSTD_getDecompressedSize, ZSTD_isError,
};

#[cfg(feature = "GZIP")]
use crate::compression_levels::GZIP_LEVEL;
#[cfg(feature = "LZMA")]
use crate::compression_levels::LZMA_LEVEL;
#[cfg(feature = "ZSTD")]
use crate::compression_levels::ZSTD_LEVEL;

use crate::enums::CompressionAlgorithm;

#[cfg(feature = "heuristics")]
use crate::heuristics::get_compressor;

pub struct Var {
    #[cfg(feature = "encryption")]
    cipher: Aes256Gcm,
    #[cfg(not(feature = "heuristics"))]
    compression: CompressionAlgorithm,
}

#[inline]
#[allow(unused_mut)]
pub fn pipeline_send(mut input: Vec<u8>, _var: &Var) -> Result<(u8, Vec<u8>), Error> {
    #[cfg(feature = "LZ4")]
    let size = input.len() as u64;

    #[cfg(feature = "heuristics")]
    let compression: &CompressionAlgorithm = &get_compressor(input.len());
    #[cfg(not(feature = "heuristics"))]
    let compression = &_var.compression;

    let mut compressed: Vec<u8> = match *compression {
        #[cfg(feature = "LZMA")]
        CompressionAlgorithm::Lzma => {
            let mut encoder = xz2::write::XzEncoder::new(Vec::new(), LZMA_LEVEL as u32);
            encoder.write_all(&input)?;
            encoder.finish()?
        }
        #[cfg(feature = "ZSTD")]
        CompressionAlgorithm::Zstd => {
            let max_size = unsafe { ZSTD_compressBound(input.len()) };
            let mut output = Vec::with_capacity(max_size);
            let err = unsafe {
                ZSTD_compress(
                    output.as_mut_ptr() as *mut c_void,
                    output.capacity(),
                    input.as_ptr() as *const c_void,
                    input.len(),
                    ZSTD_LEVEL as i32,
                )
            };
            if unsafe { ZSTD_isError(err) } != 0 {
                return Err(Error::other("Failed to compress using ZSTD"));
            }
            unsafe { output.set_len(err as usize) };
            output
        }
        #[cfg(feature = "GZIP")]
        CompressionAlgorithm::Gzip => {
            use std::io::Write;

            let mut encoder =
                GzEncoder::new(Vec::new(), flate2::Compression::new(GZIP_LEVEL as u32));
            encoder.write_all(&input)?;
            encoder.finish()?
        }
        #[cfg(feature = "LZ4")]
        CompressionAlgorithm::Lz4 => lz4_flex::compress(&input),
        _ => input,
    };

    compressed.shrink_to_fit();

    #[cfg(feature = "LZ4")]
    let mut stuff = {
        if matches!(compression, CompressionAlgorithm::Lz4) {
            let mut buffer = Vec::with_capacity(8 + compressed.len());
            buffer.extend_from_slice(&size.to_be_bytes());
            buffer.extend_from_slice(&compressed);
            buffer
        } else {
            compressed
        }
    };

    #[cfg(not(feature = "LZ4"))]
    let mut stuff = compressed;

    #[cfg(feature = "encryption")]
    {
        let mut non = [0u8; 12];
        {
            let mut rng = OsRng;
            rng.fill_bytes(&mut non);
        }
        match _var.cipher.encrypt(&non.into(), stuff.as_slice()) {
            Ok(a) => {
                stuff = {
                    let mut buffer = Vec::with_capacity(12 + a.len());
                    buffer.extend_from_slice(&non);
                    buffer.extend_from_slice(&a);
                    buffer
                };
            }
            Err(e) => return Err(Error::other(e.to_string())),
        }
    }

    Ok((compression.u8(), stuff))
}

#[inline]
pub fn pipeline_receive(compr_alg: u8, mut input: Vec<u8>, _var: &Var) -> Result<Vec<u8>, Error> {
    let compression: CompressionAlgorithm = compr_alg.into();

    #[cfg(feature = "encryption")]
    {
        if input.len() < 28 {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid encrypted data"));
        }
        let nonce_slice = &input[0..12];
        let payload = &input[12..];
        match _var.cipher.decrypt(nonce_slice.into(), payload.as_ref()) {
            Ok(dec) => input = dec,
            Err(e) => return Err(Error::other(e.to_string())),
        }
    }

    #[cfg(feature = "LZ4")]
    let _size = if matches!(compression, CompressionAlgorithm::Lz4) {
        let size = u64::from_be_bytes({
            let mut a = [0u8; 8];
            a.copy_from_slice(&input[..8]);
            a
        });
        input = input[8..].to_vec();
        size
    } else {
        0u64
    };

    let decompressed: Vec<u8> = match compression {
        #[cfg(feature = "LZMA")]
        CompressionAlgorithm::Lzma => {
            let mut decoder = xz2::read::XzDecoder::new(&input[..]);
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            output
        }
        #[cfg(feature = "ZSTD")]
        CompressionAlgorithm::Zstd => {
            let decomp_size =
                unsafe { ZSTD_getDecompressedSize(input.as_ptr() as *const c_void, input.len()) };
            if decomp_size as u64 == ZSTD_CONTENTSIZE_UNKNOWN as u64
                || decomp_size as u64 == ZSTD_CONTENTSIZE_ERROR as u64
            {
                return Err(Error::other("Failed to get ZSTD decompressed size"));
            }
            let mut output = Vec::with_capacity(decomp_size as usize);
            let err = unsafe {
                ZSTD_decompress(
                    output.as_mut_ptr() as *mut c_void,
                    decomp_size as usize,
                    input.as_ptr() as *const c_void,
                    input.len(),
                )
            };
            if unsafe { ZSTD_isError(err) } != 0 {
                return Err(Error::other("Failed to decompress using ZSTD"));
            }
            unsafe { output.set_len(err as usize) };
            output
        }
        #[cfg(feature = "GZIP")]
        CompressionAlgorithm::Gzip => {
            use flate2::read::GzDecoder;

            let mut decoder = GzDecoder::new(&input[..]);
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            output
        }
        #[cfg(feature = "LZ4")]
        CompressionAlgorithm::Lz4 => match lz4_flex::decompress(&input, _size as usize) {
            Ok(a) => a,
            Err(e) => return Err(Error::other(e.to_string())),
        },
        _ => input,
    };
    Ok(decompressed)
}

#[cfg(test)]
mod test_pipeline {
    use super::*;
    #[cfg(feature = "encryption")]
    use aes_gcm::KeyInit;
    #[cfg(not(feature = "encryption"))]
    use std::time::Instant;
    #[test]
    fn run() {
        let var = Var {
            #[cfg(feature = "encryption")]
            cipher: {
                let mut o = OsRng;
                let mut secret = [0u8; 32];
                o.fill_bytes(&mut secret);
                Aes256Gcm::new(&secret.into())
            },
            #[cfg(not(feature = "heuristics"))]
            compression: CompressionAlgorithm::get(),
        };
        let mut bts = vec![0u8; 16];
        #[cfg(feature = "encryption")]
        {
            let mut o = OsRng;
            o.fill_bytes(&mut bts);
        }
        #[cfg(not(feature = "encryption"))]
        {
            bts.clear();
            let instance = Instant::now();

            bts.extend_from_slice(&instance.elapsed().as_nanos().to_ne_bytes());
            std::thread::yield_now();
        }
        let result = {
            println!("input: {:?}", bts);
            let b = pipeline_send(bts.clone(), &var).unwrap();
            println!("algorithm: {}", b.0);
            let a = pipeline_receive(b.0, b.1, &var).unwrap();
            println!("output: {:?}", a);

            println!("tag? {}", a.len() / 16 == 2);
            a
        };
        assert!(bts == result);
    }
}
