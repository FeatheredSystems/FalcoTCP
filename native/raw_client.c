#include <openssl/crypto.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include "net.h"
#include "openssl/ssl.h"
#define BLOCKING 1
#define TLS 1

typedef struct {
    int fd;
    #if TLS
    SSL* ssl;
    SSL_CTX* ctx;
    #endif
} PrimitiveClient;

typedef struct {
    char* host;
    u_int16_t port;
    #if TLS
        char* domain;
    #endif
} PrimitiveClientSettings;

struct Packet{
    MessageHeaders headers;
    unsigned char* value;
};

int pc_create(PrimitiveClient* self, PrimitiveClientSettings settings){
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if(fd == 0){
        return -1; 
    }
    struct sockaddr_in sets = {0};
    sets.sin_family = AF_INET;
    sets.sin_port = htons(settings.port);
    inet_pton(AF_INET, settings.host, &sets.sin_addr);
    int result = connect(fd, (struct sockaddr*)(&sets), sizeof(sets));
    self->fd = fd;
    #if TLS
        SSL_CTX *ctx = SSL_CTX_new(TLS_client_method());
        SSL_CTX_set_min_proto_version(ctx, TLS1_3_VERSION);
        SSL_CTX_set_max_proto_version(ctx, TLS1_3_VERSION);
        SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
        SSL_CTX_set_default_verify_paths(ctx);
        self->ssl = SSL_new(ctx);
        self->ctx = ctx;
        SSL_set_fd(self->ssl, fd);
        SSL_set_tlsext_host_name(self->ssl, settings.domain);
        if (SSL_connect(self->ssl) <= 0) {
            return -1;
        }
    #endif
    return result;
}

static inline void serialize_message_headers(const MessageHeaders *msg, uint8_t *buf) {
    for (int i = 0; i < 8; i++) {
        buf[i] = (msg->size >> (i * 8)) & 0xFF;
    }
    buf[8] = msg->compr_alg;  
}

static inline void deserialize_message_headers(const uint8_t *buf, MessageHeaders *msg) {
    msg->size = 0;
    for (int i = 0; i < 8; i++) {
        msg->size |= ((uint64_t)buf[i]) << (i * 8);
    }
    msg->compr_alg = buf[8];
}

static inline int pc_write(PrimitiveClient *self, unsigned char *restrict buf, usize size){
    #if TLS
        return SSL_write(self->ssl, buf, size);
    #else
        return write(self->fd, buf, size);  
    #endif
}


int pc_input_request(PrimitiveClient *self, unsigned char *restrict buf, usize size, MessageHeaders headers){
    usize written = 0;
    while (written != sizeof(MessageHeaders) && written >= 0){
        unsigned char hbuf[sizeof(headers)];
        serialize_message_headers(&headers, hbuf);
        written += pc_write(self, (hbuf)+written, sizeof(headers)-written);     
    }
    written = 0;
    while(written != size && written >= 0){
        written += pc_write(self,(buf)+written,size-written);
    }
    return written;
}

static inline int pc_read(PrimitiveClient *self, unsigned char *restrict buf, usize size){
    #if TLS
        return SSL_read(self->ssl, buf, size);
    #else
        return read(self->fd, buf, size);  
    #endif
}

void pc_clean(PrimitiveClient* self){
    close(self->fd);
    #if TLS
    SSL_shutdown(self->ssl);
    SSL_free(self->ssl);
    SSL_CTX_free(self->ctx);
    EVP_cleanup();
    #endif
}
