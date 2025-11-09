#[cfg(feature = "server")]
pub mod networker;

#[cfg(any(feature = "server", feature = "client"))]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MessageHeaders {
    pub size: u64,
    pub compr_alg: u8,
}

#[cfg(any(feature = "server", feature = "client"))]
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum CompressionAlgorithm {
    None = 0,
    LZMA = 1,
    GZIP = 2,
    LZ4 = 3,
    ZSTD = 4,
}

#[cfg(any(feature = "server", feature = "client"))]
impl From<u8> for CompressionAlgorithm {
    fn from(value: u8) -> Self {
        match value {
            0 => CompressionAlgorithm::None,
            1 => CompressionAlgorithm::LZMA,
            2 => CompressionAlgorithm::GZIP,
            3 => CompressionAlgorithm::LZ4,
            4 => CompressionAlgorithm::ZSTD,
            _ => CompressionAlgorithm::None, // fallback
        }
    }
}

#[cfg(any(feature = "server", feature = "client"))]
impl From<CompressionAlgorithm> for u8 {
    fn from(value: CompressionAlgorithm) -> Self {
        value as u8
    }
}

#[cfg(any(feature = "server", feature = "client"))]
impl From<CompressionAlgorithm> for i32 {
    fn from(value: CompressionAlgorithm) -> Self {
        value as i32
    }
}

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "heuristics")]
pub mod heuristics;

#[cfg(any(feature = "falco-server", feature = "falco-client"))]
pub mod falco_pipeline;

pub mod enums;
mod test;
// ===========================================
// Compression level constants per heuristic
// ===========================================

pub mod compression_levels {
    #[cfg(feature = "LZMA")]
    pub const LZMA_LEVEL: usize = 4;
    #[cfg(feature = "GZIP")]
    pub const GZIP_LEVEL: usize = 5;
    #[cfg(feature = "ZSTD")]
    pub const ZSTD_LEVEL: usize = 6;
}

#[cfg(feature = "falco-client")]
pub mod falco_client;
