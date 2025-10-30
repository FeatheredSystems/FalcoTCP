::html::
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            background: #0b0f1a;
            color: #e2e8f0;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Helvetica', 'Arial', sans-serif;
            line-height: 1.6;
            position: relative;
        }

        .bloom-container {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100vh;
            pointer-events: none;
            z-index: 0;
            overflow: hidden;
        }

        .bloom {
            position: absolute;
            border-radius: 50%;
            filter: blur(100px);
            opacity: 0.12;
            transition: all 0.5s ease-out;
        }

        .bloom-1 {
            width: 600px;
            height: 600px;
            background: radial-gradient(circle, rgba(59, 130, 246, 0.4) 0%, rgba(59, 130, 246, 0) 70%);
            top: -10%;
            left: -5%;
        }

        .bloom-2 {
            width: 500px;
            height: 500px;
            background: radial-gradient(circle, rgba(96, 165, 250, 0.3) 0%, rgba(96, 165, 250, 0) 70%);
            top: 20%;
            right: 10%;
        }

        .bloom-3 {
            width: 700px;
            height: 700px;
            background: radial-gradient(circle, rgba(30, 64, 175, 0.35) 0%, rgba(30, 64, 175, 0) 70%);
            bottom: -15%;
            left: 15%;
        }

        .bloom-4 {
            width: 450px;
            height: 450px;
            background: radial-gradient(circle, rgba(147, 197, 253, 0.25) 0%, rgba(147, 197, 253, 0) 70%);
            top: 50%;
            right: 20%;
        }

        .bloom-5 {
            width: 550px;
            height: 550px;
            background: radial-gradient(circle, rgba(219, 234, 254, 0.2) 0%, rgba(219, 234, 254, 0) 70%);
            top: 70%;
            left: 40%;
        }

        section,header {
            position: relative;
            z-index: 1;
            max-width: 900px;
            margin: 0 auto;
            padding-left: 3rem;
            padding-right: 3rem;
        }

        h1 {
            font-size: 2.5rem;
            font-weight: 700;
            color: #f8fafc;
            margin-bottom: 1.5rem;
            margin-top: 2rem;
        }

        h2 {
            font-size: 2rem;
            font-weight: 600;
            color: #cbd5e1;
            margin-top: 3rem;
            margin-bottom: 1rem;
            padding-bottom: 0.5rem;
            border-bottom: 1px solid rgba(59, 130, 246, 0.2);
        }

        h3 {
            font-size: 1.5rem;
            font-weight: 600;
            color: #94a3b8;
            margin-top: 2rem;
            margin-bottom: 0.75rem;
        }

        p {
            color: #94a3b8;
            margin-bottom: 1rem;
            font-size: 1rem;
        }

        a {
            color: #60a5fa;
            text-decoration: none;
            transition: color 0.2s;
        }

        a:hover {
            color: #93c5fd;
            text-decoration: underline;
        }

        pre {
            background: rgba(15, 23, 42, 0.8);
            border: 1px solid rgba(59, 130, 246, 0.3);
            border-radius: 8px;
            padding: 1.5rem;
            margin: 1.5rem 0;
            overflow-x: auto;
        }

        code {
            background: rgba(15, 23, 42, 0.6);
            border: 1px solid rgba(59, 130, 246, 0.2);
            border-radius: 4px;
            padding: 0.2rem 0.4rem;
            font-family: 'Courier New', 'Consolas', monospace;
            font-size: 0.9em;
            color: #e2e8f0;
        }

        pre code {
            background: none;
            border: none;
            padding: 0;
            color: #e2e8f0;
        }

        blockquote {
            background: rgba(30, 64, 175, 0.1);
            border-left: 4px solid rgba(59, 130, 246, 0.5);
            padding: 1rem 1.5rem;
            margin: 1.5rem 0;
            color: #cbd5e1;
        }

        blockquote strong {
            color: #f8fafc;
        }

        em, i {
            color: #cbd5e1;
        }

        strong {
            color: #f8fafc;
            font-weight: 600;
        }
    .hl-keyword { color: #c678dd; font-weight: 500; }
    .hl-type    { color: #e5c07b; }
    .hl-func    { color: #61afef; }
    .hl-string  { color: #98c379; }
    .hl-number  { color: #d19a66; }
    .hl-comment { color: #5c6370; font-style: italic; }
    .hl-field   { color: #abb2bf; }
    </style>

    <div class="bloom-container">
        <div class="bloom bloom-1"></div>
        <div class="bloom bloom-2"></div>
        <div class="bloom bloom-3"></div>
        <div class="bloom bloom-4"></div>
        <div class="bloom bloom-5"></div>
    </div>
    <script>
        const blooms = document.querySelectorAll('.bloom');

        window.addEventListener('scroll', () => {
            const scrolled = window.pageYOffset;
            const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
            const scrollPercent = scrolled / maxScroll;

            blooms.forEach((bloom, i) => {
                const speed = (i + 1) * 0.15;
                const translateY = scrolled * speed * 0.5;
                const translateX = Math.sin(scrolled * 0.0005 + i) * 30;
                
                const opacity = 0.12 - (scrollPercent * 0.04);
                const blur = 100 + (scrollPercent * 20);
                
                bloom.style.transform = `translate(${translateX}px, ${translateY}px)`;
                bloom.style.opacity = Math.max(0.06, opacity);
                bloom.style.filter = `blur(${blur}px)`;
            });
        });
    </script>
<header>
::html::

# FalcoTCP
A network handler designed for a trusted end-to-end connection, such as a microservice-to-microservice connection. It uses a pre-shared key for on-the-fly encryption, for performance's sake.

::html::
</header>
<main>
<section id="Networker">
::html::

# Networker
This is the server engine, it has a clock-cycle that handle connections.

## Headers
Every request must include headers, they tell how long is the payload, and what compression algorithm is being used (If any). The headers size is 9 in bytes, the first 8 bytes being the 64-bit unsigned integer that represent the payload size, and a single byte flagging the compression algorithm. Both request, and response, use this header format.
::html::
</section>
<section id="Cycle">
::html::
## Cycle
The engine is essentially, a state manager that operate in cycles. Each one updates the clients it keeps, processing requests and responses at the socket level. A client can be in 11 different states, which are: *<a href="#NonExistent">NonExistent</a>, <a href="#Idle">Idle</a>, <a href="#HeadersReading">Headers Reading</a>, <a href="#Reading">Reading</a>, <a href="#Finished-Reading">Finished reading</a>, <a href="#Available">Available</a>, <a href="#Processing">Processing</a>, <a href="#Ready">Ready</a>, <a href="#WritingSock">Writing Sock</a>, <a href="#Kill">Kill</a>,* and *<a href="#Finished-Writing">Finished Writing</a>*.
::html::
</section>
<section id="NonExistent">
::html::
### NonExistent
This state is given to a client that is just allocated, has no socket binded to it, basically a spot for addressing a new connection. Every client, after getting the state <a href="#Kill">*Kill*</a> is stated as such.
::html::
</section>
<section id="Idle">
::html::
### Idle
Describes a connection which is openned, but with no processing being done from it. If a connection remains the state for more than the magic number of 1200 seconds (20 minutes), it gets the state <a href="#Kill">*Kill*</a>.
::html::
</section>
<section id="HeadersReading">
::html::
### Headers reading
To check whether a connection made a request or not, it is set to this state (`Finished_H` internally). If bytes equal to the request method size get readen from that client socket, it is set to the state <a href="#Reading">Reading</a>, <a href="#Idle">Idle</a> otherwise.
::html::
</section>
<section id="Reading">
::html::
### Reading
A state that is given to the client that successfully get a request, and have its headers readen. It request to read from the socket, the byte count set in the headers. To guarantee all bytes will be readen, the connection is set to the state <a href="#Finished-Reading">Finished reading</a>.
::html::
</section>
<section id="Finished-Reading">
::html::
### Finished reading
If the read from <a href="#Reading">Reading</a> fail to retrieve the expected count, the client is locked into this state until all bytes requested get readen. That said, the state read the socket until there are no bytes remaining to be readen, seting the client to <a href="#Available">Available</a> if so.
::html::
</section>
<section id="Available">
::html::
### Available
As a client reach this state, the server handler in the rust end can request it and set it to <a href="#Processing">Processing</a>, get the request content, process it, embed a response, and send. As the response is linked to the client, its state is set to <a href="#Ready">Ready</a>.
::html::
</section>
<section id="Processing">
::html::
### Processing
A state that tells that the Rust end is computing this request.
::html::
</section>
<section id="#Ready">
::html::
### Ready
As a client has this state, the networker acknowledge it owns the client and can start the socket response writing process. Setting the state to <a href="#WritingSock">Writing Sock</a>.
::html::
</section>
<section id="WritingSock">
::html::
### Writing sock
The write version of <a href="#Reading">Reading</a>, tries to write, setting it on a state to rewrite in case not all bytes get written at once. The <a href="#FinishedWriting">Finished writing</a> state.
::html::
</section>
<section id="FinishedWriting">
::html::
### Finished writing
Rewrite remaining bytes, until none has left to be written. As all get written, the cycle rollback, state being set to <a href="#Idle">Idle</a>.
::html::
</section>
<section id="Kill">
::html::
### Kill
In cases of malfunction, the client is killed, related buffers and file descriptors are freed/closed. The state being set back to <a href="#NonExistent">NonExistent</a>.
::html::
</section>
<section id="Rust-Networker-Implementation">
::html::
# Rust Networker Implementation
<blockquote>
  <strong>Note:</strong> The content below is taken directly from the <code>Networker</code> struct documentation.
</blockquote>
::html::
<pre><code>
<span class="hl-keyword">pub</span> <span class="hl-keyword">struct</span> <span class="hl-type">Networker</span> {
    <span class="hl-field">primitive_self</span>: <span class="hl-type">RawNetworker</span>,
    <span class="hl-field">mutex</span>: <span class="hl-type">Mutex&lt;()&gt;</span>,
    <span class="hl-field">initilized</span>: <span class="hl-type">u8</span>,
}
</code></pre>
::html::
`Networker` wraps a C implementation that uses Linux's io_uring to handle multiple client connections. The server operates in cycles, where each cycle processes pending I/O operations for all connected clients.
## Structure
The networker allocates a fixed number of client slots during initialization. Each slot can hold one client connection and tracks that connection's state through the request-response lifecycle.
## Concurrency
This structure implements `Send` and `Sync`. Internal operations use a mutex to coordinate access to the underlying C structures and client state.
## Features
When the `tokio-runtime` feature is enabled, methods like `cycle()` and `get_client()` become async and integrate with the Tokio runtime. Without this feature, these methods are synchronous.
## Panics
Methods `cycle()` and `get_client()` panic if called on an uninitialized `Networker`. Use `Networker::new()` to initialize before calling these methods. `Networker::default()` creates an uninitialized instance.
## Safety
This structure wraps C FFI calls and manages raw pointers. Safety is maintained through state management and the internal mutex.
::html::
</section>
</main>
::html::
