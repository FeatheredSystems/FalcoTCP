# FalcoTCP
This is a secure connection protocol for communication between trusted endpoints like server-to-microservices or server-database interactions. The implementation is very lightweight and has minimal overhead alongside a straightforward API.

## Protocol
Works with the server listening to the host address. When a client tries to connect, it sends a chunk of bytes encrypted with AES-GCM-256. If the server decrypts the request, the connection is successfully established; the TCP socket is shut down otherwise.

After connecting, every connection has a lifetime of 60 seconds, the countdown being reset after any sort of interaction, it being able to be either a ping or message.

When the server receives a message from the client, it decrypts the message (everything in the network is encrypted by default) and calls a function (message handler) which gets these bytes as parameters and returns bytes that are encrypted and sent back to the client. It decrypts the bytes from the server and returns them to the client runtime.

## Implementations
Currently, it has only a single connection handler in Rust, but in the future, I might implement it for both Python and Go.

## Documentation
[not ready yet]

## Contributions
Contributions are not accepted for this project.

## Feedback / Security reports
If you find either a security problem, bug, or want to give feedback or suggest something, email me at the project email address.  
Email: falcotcp@gmail.com