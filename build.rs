fn main() {
    #[cfg(feature = "server")]
    {
        println!("cargo:rustc-link-search=native=/usr/lib");
        println!("cargo:rerun-if-changed=native/numbers.h");
        println!("cargo:rerun-if-changed=native/net.h");
        println!("cargo:rerun-if-changed=native/net.c");

        if std::env::var("CARGO_FEATURE_SERVER").is_ok() {
            let mut build = cc::Build::new();
            build.file("native/net.c").include("native");

            let profile = std::env::var("PROFILE").unwrap_or_default();
            if profile == "release" {
                build.flag("-O3");
            }

            #[cfg(not(feature = "dynamic-uring-link"))]
            println!("cargo:rustc-link-lib=static=uring");
            #[cfg(feature = "dynamic-uring-link")]
            println!("cargo:rustc-link-lib=uring");
            build.compile("networker");
        }
    }
}
