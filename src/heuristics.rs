use crate::enums::CompressionAlgorithm;

#[inline]
#[allow(unreachable_code)]
pub const fn get_compressor(_size: usize) -> CompressionAlgorithm{
    // heubias-performance
    #[cfg(all(feature="heubias-performance",not(feature="heubias-ratio")))]
    {
        #[cfg(feature = "LZ4")]
        return CompressionAlgorithm::Lz4;
        #[cfg(not(feature = "LZ4"))]
        {
            #[cfg(feature = "ZSTD")]
            return CompressionAlgorithm::Zstd;
            #[cfg(not(feature = "ZSTD"))]
            {
                #[cfg(feature = "GZIP")]
                return CompressionAlgorithm::Gzip;
                #[cfg(not(feature = "GZIP"))]
                return CompressionAlgorithm::None;
            }
        }
    }
    #[cfg(all(feature="heubias-ratio",not(feature="heubias-performance")))]
    {
        let size = _size;
        #[cfg(any(feature = "LZMA", feature = "GZIP", feature = "ZSTD", feature = "LZ4"))]
        if size < 10485760  {
            #[cfg(feature = "LZMA")]
            {
                return CompressionAlgorithm::Lzma;
            }
            #[cfg(feature = "GZIP")]
            {
                return CompressionAlgorithm::Gzip;
            }
            #[cfg(feature = "ZSTD")]
            {
                return CompressionAlgorithm::Zstd;
            }
            #[cfg(feature = "LZ4")]
            {
                return CompressionAlgorithm::Lz4;
            }
        }
        #[cfg(any(feature = "GZIP", feature = "ZSTD", feature = "LZ4"))]
        if size > 10485760 && size < 209715200 {
            #[cfg(feature = "GZIP")]
            {
                return CompressionAlgorithm::Gzip;
            }
            #[cfg(feature = "ZSTD")]
            {
                return CompressionAlgorithm::Zstd;
            }
            #[cfg(feature = "LZ4")]
            {
                return CompressionAlgorithm::Lz4;
            }
        }
        #[cfg(any(feature = "ZSTD", feature = "GZIP"))]
        if size >= 209715200 && size < 314572800 {
            #[cfg(feature = "ZSTD")]
            {
                return CompressionAlgorithm::Zstd;
            }
            #[cfg(feature = "GZIP")]
            {
                return CompressionAlgorithm::Gzip;
            }
        }
        #[cfg(feature = "LZ4")]
        if size >= 314572800 {
            // 300 mib
            #[cfg(feature = "LZ4")]
            {
                return CompressionAlgorithm::Lz4;
            }
        }
        CompressionAlgorithm::None

    }
    #[cfg(any(all(feature="heubias-ratio",feature="heubias-performance"),not(any(all(feature="heubias-ratio",feature="heubias-performance")))))]
    {
        #[cfg(any(feature = "LZMA", feature = "ZSTD", feature = "GZIP", feature = "LZ4"))]
        let size = _size;
        #[cfg(any(feature = "LZMA", feature = "ZSTD", feature = "GZIP", feature = "LZ4"))]
        if size < 10485760 {
            #[cfg(feature = "LZMA")]
            {
                return CompressionAlgorithm::Lzma;
            }
            #[cfg(feature = "ZSTD")]
            {
                return CompressionAlgorithm::Zstd;
            }
            #[cfg(feature = "GZIP")]
            {
                return CompressionAlgorithm::Gzip;
            }
            #[cfg(feature = "LZ4")]
            {
                return CompressionAlgorithm::Lz4;
            }
        }
        #[cfg(any(feature = "ZSTD", feature = "GZIP", feature = "LZMA"))]
        if size >= 10485760 && size < 209715200 {
            #[cfg(feature = "ZSTD")]
            {
                return CompressionAlgorithm::Zstd;
            }
            #[cfg(feature = "LZMA")]
            {
                return CompressionAlgorithm::Lzma;
            }
            #[cfg(feature = "GZIP")]
            {
                return CompressionAlgorithm::Gzip;
            }
        }
        #[cfg(any(feature = "ZSTD", feature = "LZMA"))]
        if size >= 209715200 {
            #[cfg(feature = "ZSTD")]
            {
                return CompressionAlgorithm::Zstd;
            }
            #[cfg(feature = "LZMA")]
            {
                return CompressionAlgorithm::Lzma;
            }
        }

        CompressionAlgorithm::None

    }
}

