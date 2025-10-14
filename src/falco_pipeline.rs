use aes_gcm::Aes256Gcm;
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, OsRng};
#[cfg(all(feature = "ZSTD", not(feature = "LZMA")))]
use std::ffi::c_void;
use std::io::{Error, ErrorKind};

#[cfg(feature = "GZIP")]
use std::io::Read;
#[cfg(feature = "LZMA")]
use std::{ffi::c_void, ops::Deref};
#[cfg(feature = "ZSTD")]
use zstd::zstd_safe::zstd_sys::{
    ZSTD_CONTENTSIZE_ERROR, ZSTD_CONTENTSIZE_UNKNOWN, ZSTD_compress, ZSTD_decompress,
    ZSTD_getDecompressedSize, ZSTD_isError,
};

#[cfg(feature = "GZIP")]
use crate::compression_levels::GZIP_LEVEL;
#[cfg(feature = "LZMA")]
use crate::compression_levels::LZMA_LEVEL;
#[cfg(feature = "ZSTD")]
use crate::compression_levels::ZSTD_LEVEL;

use crate::{enums::CompressionAlgorithm, heuristics::get_compressor};

pub struct Var {
    #[cfg(feature = "encryption")]
    cipher: Aes256Gcm,
}
#[inline]
pub fn pipeline_send(input: Vec<u8>, var: &Var) -> Result<(u8, Vec<u8>), Error> {
    #[cfg(feature = "ZSTD")]
    let mut input = input;

    #[cfg(feature = "LZ4")]
    let size = input.len() as u64;
    let compression: CompressionAlgorithm = get_compressor(input.len());
    let mut compressed: Vec<u8> = match compression {
        #[cfg(feature = "LZMA")]
        CompressionAlgorithm::Lzma => xz2::write::XzEncoder::new(&mut input, LZMA_LEVEL as u32)
            .finish()?
            .deref()
            .deref()
            .to_vec(),
        #[cfg(feature = "ZSTD")]
        CompressionAlgorithm::Zstd => {
            let mut output = Vec::with_capacity(input.len());
            let err = unsafe {
                ZSTD_compress(
                    output.as_mut_ptr() as *mut c_void,
                    output.capacity(),
                    input.as_mut_ptr() as *mut c_void,
                    input.len(),
                    ZSTD_LEVEL as i32,
                )
            };
            if unsafe { ZSTD_isError(err) } != 0 {
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to compress using ZSTD",
                ));
            }
            unsafe { output.set_len(err as usize) };
            output
        }
        #[cfg(feature = "GZIP")]
        CompressionAlgorithm::Gzip => {
            let mut output = Vec::with_capacity(input.len());
            match flate2::Compress::new(flate2::Compression::new(GZIP_LEVEL as u32), false)
                .compress(&input, &mut output, flate2::FlushCompress::Full)
            {
                Ok(a) => match a {
                    flate2::Status::Ok => (),
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Failed to compress using GZIP",
                        ));
                    }
                },
                Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
            };

            output
        }
        #[cfg(feature = "LZ4")]
        CompressionAlgorithm::Lz4 => lz4_flex::compress(&input),
        _ => input,
    };

    compressed.shrink_to_fit();

    #[cfg(feature = "LZ4")]
    let mut stuff = {
        let mut buffer = Vec::with_capacity(8 + compressed.len());
        buffer.extend_from_slice(&size.to_be_bytes());
        buffer.extend_from_slice(&compressed);
        buffer
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
        match var.cipher.encrypt(&non.into(), stuff.as_slice()) {
            Ok(a) => {
                stuff = {
                    let mut buffer = Vec::with_capacity(12 + a.len());
                    buffer.extend_from_slice(&non);
                    buffer.extend_from_slice(&a);
                    buffer
                };
            }
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }

    Ok((compression.u8(), stuff))
}
#[inline]
pub fn pipeline_receive(compr_alg: u8, mut input: Vec<u8>, var: &Var) -> Result<Vec<u8>, Error> {
    #[cfg(feature = "LZ4")]
    let _size = u64::from_be_bytes({
        let mut a = [0u8; 8];
        a.copy_from_slice(&input[..8]);
        a
    });
    #[cfg(feature = "encryption")]
    {
        if input.len() < 28 {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid encrypted data"));
        }
        let nonce_slice = &input[0..12];
        let payload = &input[12..];
        match var.cipher.decrypt(nonce_slice.into(), payload.as_ref()) {
            Ok(dec) => input = dec,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }
    #[cfg(feature = "LZ4")]
    let input = input[8..].to_vec();
    let compression: CompressionAlgorithm = compr_alg.into();
    let decompressed: Vec<u8> = match compression {
        #[cfg(feature = "LZMA")]
        CompressionAlgorithm::Lzma => {
            let decoder = xz2::read::XzDecoder::new(&input[..]);
            decoder.into_inner().to_vec()
        }
        #[cfg(feature = "ZSTD")]
        CompressionAlgorithm::Zstd => {
            let decomp_size =
                unsafe { ZSTD_getDecompressedSize(input.as_ptr() as *const c_void, input.len()) };
            if decomp_size as u64 == ZSTD_CONTENTSIZE_UNKNOWN as u64
                || decomp_size as u64 == ZSTD_CONTENTSIZE_ERROR as u64
            {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to get ZSTD decompressed size",
                ));
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
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to decompress using ZSTD",
                ));
            }
            unsafe { output.set_len(err as usize) };
            output
        }
        #[cfg(feature = "GZIP")]
        CompressionAlgorithm::Gzip => {
            let mut decoder = flate2::read::DeflateDecoder::new(&input[..]);
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            output
        }
        #[cfg(feature = "LZ4")]
        CompressionAlgorithm::Lz4 => match lz4_flex::decompress(&input, _size as usize) {
            Ok(a) => a,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        },
        _ => input,
    };
    Ok(decompressed)
}
#[cfg(test)]
mod test_pipeline {
    use aes_gcm::KeyInit;

    use super::*;
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
        };
        let mut bts = vec![0u8; 1024];
        let mut o = OsRng;
        o.fill_bytes(&mut bts);
        let result = {
            let b = pipeline_send(bts.clone(), &var).unwrap();
            pipeline_receive(b.0, b.1, &var).unwrap()
        };
        assert!(bts == result);
    }
}
