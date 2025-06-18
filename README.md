# FalcoTCP
This is a secure connection protocol for comunication between trusted endpoints like server-to-microservices or server-database interactions, the implementation is very lightweight and have minimal overhead alongside with a straightforward API.

## Protocol
Works with the server listening to the host address, when a client try to connect it sends a chunk of bytes encrypted with AES-GCM-256, if the server decrypt the request the connection is successfuly stablished, the TCP socket it shutdown otherwise.
After connecting, every connection have a life time of 60 seconds, the count down being reset after any sort of interaction, it being able to be either a ping or message.
When the server receives a message from the client it decrypt the message (everything in the network is encrypted by default) and calls a function(message handler) which gets these bytes as parameters and returns bytes that are encrypted and sent back to the client. It decrypts the bytes from the server and returns to the client runtime.

## Implementations
Currently it have only a single connection handler in rust, but in the future I might implement it for both python and go.

## Documentation
[not ready yet]

## Contributions
Contributions are not accepted for this project.

## Feedback / Security reports
If you find either a security problem, bug or want to feedback or suggest something, email me at the project email address. 
Email: falcotcp@gmail.com