#[cfg(feature = "server")]
pub mod networker;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "heuristics")]
pub mod heuristics;

#[cfg(any(feature = "falco-server", feature = "falco-client"))]
pub mod falco_pipeline;

pub mod enums;

// ===========================================
// Compression level constants per heuristic
// ===========================================


#[cfg(feature="heuristics")]
pub mod compression_levels {
    pub const LZMA_LEVEL: usize = 4;
    pub const GZIP_LEVEL: usize = 5;
    pub const ZSTD_LEVEL: usize = 6;
}

#[cfg(feature = "falco-client")]
pub mod falco_client;
