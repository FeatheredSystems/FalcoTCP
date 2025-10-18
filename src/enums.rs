pub enum CompressionAlgorithm {
    None,
    #[cfg(feature = "LZMA")]
    Lzma,

    #[cfg(feature = "ZSTD")]
    Zstd,

    #[cfg(feature = "GZIP")]
    Gzip,
    #[cfg(feature = "LZ4")]
    Lz4,
}

impl CompressionAlgorithm {
    pub fn u8(self) -> u8 {
        match self {
            #[cfg(feature = "LZMA")]
            CompressionAlgorithm::Lzma => 1,
            #[cfg(feature = "ZSTD")]
            CompressionAlgorithm::Zstd => 4,
            #[cfg(feature = "GZIP")]
            CompressionAlgorithm::Gzip => 2,
            #[cfg(feature = "LZ4")]
            CompressionAlgorithm::Lz4 => 3,
            _ => 0,
        }
    }
}
impl From<u8> for CompressionAlgorithm {
    fn from(s: u8) -> CompressionAlgorithm {
        match s {
            #[cfg(feature = "LZMA")]
            1 => CompressionAlgorithm::Lzma,
            #[cfg(feature = "ZSTD")]
            4 => CompressionAlgorithm::Zstd,
            #[cfg(feature = "GZIP")]
            2 => CompressionAlgorithm::Gzip,
            #[cfg(feature = "LZ4")]
            3 => CompressionAlgorithm::Lz4,

            _ => CompressionAlgorithm::None,
        }
    }
}

impl Clone for CompressionAlgorithm {
    fn clone(&self) -> CompressionAlgorithm {
        *self
    }
}
impl Copy for CompressionAlgorithm {}

impl CompressionAlgorithm {
    /// Return one of the available compression algorithm
    #[allow(unreachable_code)]
    pub fn get() -> Self {
        #[cfg(feature = "LZMA")]
        return CompressionAlgorithm::Lzma;
        #[cfg(feature = "ZSTD")]
        return CompressionAlgorithm::Zstd;
        #[cfg(feature = "GZIP")]
        return CompressionAlgorithm::Gzip;
        #[cfg(feature = "LZ4")]
        return CompressionAlgorithm::Lz4;

        CompressionAlgorithm::None
    }
}
