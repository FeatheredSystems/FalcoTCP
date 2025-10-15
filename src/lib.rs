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

#[cfg(all(feature = "heuristics", feature = "heubias-performance"))]
pub mod compression_levels {
    pub const LZMA_LEVEL: usize = 1;
    pub const GZIP_LEVEL: usize = 1;
    pub const ZSTD_LEVEL: usize = 1;
}

#[cfg(all(feature = "heuristics", feature = "heubias-ratio"))]
pub mod compression_levels {
    pub const LZMA_LEVEL: usize = 9;
    pub const GZIP_LEVEL: usize = 9;
    pub const ZSTD_LEVEL: usize = 19;
}

#[cfg(all(
    feature = "heuristics",
    not(any(feature = "heubias-ratio", feature = "heubias-performance"))
))]
pub mod compression_levels {
    pub const LZMA_LEVEL: usize = 4;
    pub const GZIP_LEVEL: usize = 5;
    pub const ZSTD_LEVEL: usize = 6;
}

#[cfg(feature = "falco-client")]
pub mod falco_client;
