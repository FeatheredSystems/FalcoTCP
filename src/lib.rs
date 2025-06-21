//! # FalcoTCP
//!
//! FalcoTCP is a Rust implementation of the FalcoTCP protocol, providing both server and client functionalities.
//!
//! ## Installation
//!
//! To use FalcoTCP in your project, add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! falcotcp = "0.1.0"
//! ```
//!
//! Alternatively, you can use the following command:
//!
//! ```sh
//! cargo add falcotcp
//! ```
//!
//! ## Features
//!
//! The `falcotcp` crate provides different runtime features that can be enabled based on your needs:
//!
//! - `tokio-runtime`: Uses the Tokio runtime.
//! - `async-std-runtime`: Uses the async-std runtime.
//! - `thread` (default): Uses standard threads.
//!
//! To use a specific runtime, specify it in your `Cargo.toml`. For example, to use the Tokio runtime:
//!
//! ```toml
//! [dependencies]
//! falcotcp = { version = "0.1.0", default-features = false, features = ["tokio-runtime"] }
//! ```
//!
//! **Note**: The `default-features = false` property is required when using a runtime different from the default (`thread`).
//!
//! ## Server
//!
//! To implement a server, use the `Server` structure. The server starts running immediately upon creation via `Server::new`.
//!
//! ### Parameters
//!
//! - `host`: A string representing the host address (e.g., `"127.0.0.1:8000"`).
//! - `password`: A 32-byte array used for authentication.
//! - `message_handler`: A boxed closure that handles incoming messages. It takes a `Vec<u8>` and returns a `Vec<u8>`. Must implement `Send`, `Sync`, and have a `'static` lifetime.
//! - `workers`: The number of worker threads to use.
//!
//! ### Example
//!
//! This example demonstrates server initialization using the default runtime (`thread`):
//!
//! ```rust
//! use falcotcp::Server;
//!
//! const EXAMPLE_PASSWORD: [u8; 32] = [
//!     0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C, 0x65, 0x5F,
//!     0x70, 0x61, 0x73, 0x73, 0x77, 0x6F, 0x72, 0x64,
//!     0x5F, 0x31, 0x32, 0x33, 0x21, 0x40, 0x23, 0x24,
//!     0x25, 0x5E, 0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B,
//! ];
//!
//! fn main() {
//!     let message_handler = Box::new(|parameter: Vec<u8>| {
//!         parameter // Echoes the parameter; apply your logic here
//!     });
//!     if let Err(e) = Server::new(
//!         "127.0.0.1:8000".to_string(),
//!         EXAMPLE_PASSWORD,
//!         message_handler,
//!         2, // Number of worker threads
//!     ) {
//!         eprintln!("Failed to start server: {:?}", e);
//!     }
//! }
//! ```
//!
//! ## Client
//!
//! The client is non-blocking and requires the server's address and password to connect. Ensure the password matches the server's; otherwise, the connection will be terminated.
//!
//! ### Parameters for `Client::new`
//!
//! - `address`: The server's address (e.g., `"127.0.0.1:8000"`).
//! - `password`: A 32-byte array used for authentication.
//! - `timeout`: (Optional, only for the `thread` feature) A `u64` representing the timeout in seconds.
//!
//! ### Usage
//!
//! After connecting, you can send messages to the server using `client.message`. The connection lasts for 60 seconds; send a message or ping within that time to keep it alive. It is recommended to ping the server every 30 seconds.
//!
//! **Note**: There is no graceful error handling; the protocol assumes proper management of both server and client.
//!
//! ### Example
//!
//! This example demonstrates client initialization using the default runtime (`thread`):
//!
//! ```rust
//! use falcotcp::Client;
//!
//! const EXAMPLE_PASSWORD: [u8; 32] = [
//!     0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C, 0x65, 0x5F,
//!     0x70, 0x61, 0x73, 0x73, 0x77, 0x6F, 0x72, 0x64,
//!     0x5F, 0x31, 0x32, 0x33, 0x21, 0x40, 0x23, 0x24,
//!     0x25, 0x5E, 0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B,
//! ];
//!
//! fn main() {
//!     match Client::new("127.0.0.1:8000", EXAMPLE_PASSWORD) {
//!         Ok(mut client) => {
//!             match client.message(vec![8u8; 10]) { // Sends 10 bytes to the server
//!                 Ok(response) => {
//!                     println!("{:?}", response); // Print the byte response
//!                 },
//!                 Err(e) => {
//!                     eprintln!("err:{:?}", e);
//!                 }
//!             }
//!         },
//!         Err(e) => {
//!             eprintln!("failed to initialize:{:?}", e);
//!         }
//!     }
//! }
//! ```

#[cfg(feature = "thread")]
mod thread_impl;
#[cfg(feature = "tokio-runtime")]
mod tokio_impl;
#[cfg(feature = "async-std-runtime")]
mod asyncstd_impl;

#[cfg(feature = "thread")]
pub use thread_impl::*;
#[cfg(feature = "tokio-runtime")]
pub use tokio_impl::*;
#[cfg(feature = "async-std-runtime")]
pub use asyncstd_impl::*;
