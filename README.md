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
The software was written with performance in mind; it may be faster with other handlers. But I would rather not mention the "how good the project is" type of claims; they are sensitive to failures. Like a subtle bug, for instance, that said, expect it to be brutally fast, but in your workload, you should measure whether it is truly useful. Most of the time, performance isn't the only important thing to think about when calculating trade-offs.

Thanks for reading this, and for visiting this work.

### Future packages
I am intending to write a Rust package and maybe a C one, no python, go, or whatever other language. If you need a version for these tools, you may get in future an interface from the C written ones. All not focused to work with the language semantics (Go, and Python tend to have), but with the actual functionality.
