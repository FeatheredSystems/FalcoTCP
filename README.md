# FalcoTCP
This is a connection protocol for communication between trusted endpoints, such as server-to-microservice or server-database interactions. The implementation is very lightweight and has minimal overhead alongside a straightforward API.

## Protocol
Works with the server listening to the host address. When a client tries to connect, it sends a chunk of bytes encrypted with AES-GCM-256. If the server decrypts the request, the connection is successfully established; the TCP socket is shut down otherwise.

After connecting, every connection has a lifetime of 60 seconds, the countdown being reset after any sort of interaction, which can be either a ping or a message.

When the server receives a message from the client, it decrypts the message (everything in the network is encrypted by default) and calls a function (message handler) which gets these bytes as parameters and returns the encrypted message and sends it back to the client. It decrypts the bytes from the server and returns them to the client runtime.

## Implementations
Currently, it has only a single connection handler in Rust, but I may implement it for both Python and Go in the future.

## Documentation
[https://falcotcp-docs.pages.dev/](https://falcotcp-docs.pages.dev/)

## Contributions
As HTTP, FalcoTCP is a protocol, a way of interacting with networking. That said, you may and are free to create things with it. But, this specific repository is restricted to contributions due to curatorial reasons since most FeatheredSystems projects have it as a dependency. If you think a change to this repo or protocol might be interesting, please provide feedback.

## Protocol authorship
The protocol is officially maintained by FeatheredSystems, even while it is open source, and we allow the community to create their own versions. Any official release of the protocol, per se, will be done within FeatheredSystems.

## License info
This repository is under the Apache-2.0 license; the protocol itself is under the MIT license.

## Feedback / Security reports
If you find either a security problem, bug, want to give feedback, or suggest something, email me at the project email address.
Email: falcotcp@gmail.com
You can also create an Issue as long as you are not exposing any security vulnerability. For such cases, reach me at the email.
