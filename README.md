# FalcoTCP
A network handler designed for a trusted end-to-end connection, such as a microservice-to-microservice connection. It uses a pre-shared key for on-the-fly encryption, for performance's sake.

## Compatibility 
Servers only work on Linux systems with a kernel version that supports IO Uring (5.1+), clients use the Rust standard library networking. That said, clients are portable wherever Rust is supported.

## Contributions
Raise a pull request, send me an email with suggestions (At this address: "falcotcp@proton.me"), raise an issue, or reach me anywhere coherent.

## Security
It relies on the pre-shared key. That being said, both ends must handle the key properly.

If you find a sensitive spot, a vulnerability, send me an email to the mentioned endpoint (This address: "falcotcp@proton.me").

## Suggestions
Appreciated, to send yours, send me an email at the address I mentioned (falcotcp@proton.me) several times, raise an issue, or create a discussion.

## Usability
I tried making the interface as friendly as possible. If you find something awkward, raise a suggestion (through the mentioned methods).

## Docs and Installation
Work in progress.

### Personal considerations
This project prioritizes performance. It is still under development and has not been benchmarked yet. Benchmarking will be performed when TLS support is implemented.

Thanks for reading this, and for visiting this work.
