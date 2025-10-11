#include <asm-generic/errno-base.h>
#include <asm-generic/errno.h>
#include <bits/types.h>
#include <errno.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <liburing/io_uring.h>
#include <string.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>   
#include <arpa/inet.h>    
#include <time.h>
#include <unistd.h>       
#include <fcntl.h>      
#include <liburing.h>

#define u64 u_int64_t
#define u8 u_int8_t
#define usize size_t


enum State{
    NonExistent = 0,
    Idle = 1,
    HeadersReaden = 2,
    Finished_H = 3,
    Reading = 4,
    Finished_R = 5,
    Available = 6,
    Processing = 7,
    Ready = 8,
    WrittingSock = 9,
    Kill = 10,
};

typedef struct {
    u64 size;
    u8 compr_alg;
} MessageHeaders;
typedef struct{
    int sock;
    unsigned char* request;
    unsigned char* response;
    MessageHeaders req_headers; 
    u64 response_size;
    usize recv_offset;
    usize writev_offset;
    u64 id; 
    int state;
    u64 activity;
    u64 capacity;
} Client;


// Helps the rust interface to define whether it can or cannot decompress the given input
enum CompressionAlgorithm{
    None = 0,
    LZMA = 1,
    GZIP = 2,
    LZ4 = 3,
    ZSTD = 4,
};
enum Operation{
    OP_SocketAcc = 0,
    OP_Read = 1,
    OP_Write = 2,
    OP_Close = 3,
};

struct NetworkerSettings{
    char host[12];
    unsigned short port;
    unsigned short max_queue;
    unsigned short max_clients;
};

typedef struct {
    int initiated;
    int sock;
    u64 client_num;
    Client* clients;
    struct io_uring* ring;
    u64* author_log;
} Networker;

int start(Networker* self, struct NetworkerSettings settings){
    if (self->initiated > 0){
        return 0;
    }
    
    int sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock < 0){
        return -1;
    }
    
    struct  sockaddr_in sockad;
    sockad.sin_family = AF_INET;
    sockad.sin_port = htons(settings.port);
    inet_pton(AF_INET, settings.host, &sockad.sin_addr);
    int _l = bind(sock, (struct sockaddr*)&sockad, sizeof(sockad));
    if (_l < 0){
        return errno;
    }

    int _l1 = listen(sock, settings.max_queue);
    if (_l1 < 0){
        return errno;
    }
    self->sock = sock;
    self->initiated = 1;

    self->clients = (Client*)calloc(self->client_num, sizeof(Client));
    if(!self->clients){
        return ENOMEM;  
    }
    for(usize i = 0; i < self->client_num; i++){
        self->clients[i].id = i;
        self->clients[i].request = NULL;
        self->clients[i].response = NULL;
    }

    self->author_log = malloc(self->client_num*sizeof(u64));
    if(!self->author_log){
        return ENOMEM;
    }

    {
        int res = io_uring_queue_init(self->client_num, self->ring, 0);
        if(res < 0) {
            return res;
        }
    }

    return 0;
}

int proc(Networker* self){
    #define ring *self->ring

    u64 *author_log = self->author_log;
    u64 calls = 0;

    #define LOG_AUTHOR \
    author_log[calls] = i;\

    #define REGISTER \
    LOG_AUTHOR \
    calls++ \

    u64 now = time(NULL);
    
    for(int i = 0; i < self->client_num; i++){
        if(self->clients[i].state == NonExistent){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            io_uring_prep_accept(sqe, self->sock, NULL,NULL, 0);
            sqe->user_data = OP_SocketAcc;
            REGISTER; 
            continue;
        }        
        if(self->clients[i].state == Idle){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            if(now-self->clients[i].activity > 3600){
                sqe->user_data = OP_Close;
                io_uring_prep_close(sqe, self->clients[i].sock);
                self->clients[i].state = NonExistent;
                free(self->clients[i].response);
                free(self->clients[i].request);
            }
            sqe->user_data = OP_Read;
            self->clients[i].state = Finished_H;
            io_uring_prep_read(sqe,self->clients[i].sock, (char*)(&self->clients[i].req_headers)+self->clients[i].recv_offset, sizeof(MessageHeaders), 0);
            REGISTER;
            continue;
        }

        if(self->clients[i].state == Finished_H){
            if(self->clients[i].recv_offset == sizeof(MessageHeaders)){
                self->clients[i].recv_offset = 0;
                self->clients[i].state = Reading;
            }else{
                self->clients[i].state = Idle;
                continue;
            }
        }

        if(self->clients[i].state == Reading){
            if(self->clients[i].capacity < self->clients[i].req_headers.size || self->clients[i].request == NULL){
                self->clients[i].request = malloc(self->clients[i].req_headers.size);
                self->clients[i].capacity = self->clients[i].req_headers.size;
            }
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            sqe->user_data = Finished_R;
            io_uring_prep_read(sqe,self->clients[i].sock,self->clients[i].request + self->clients[i].recv_offset,self->clients[i].req_headers.size,0);
            REGISTER;
            continue;
        }

        if(self->clients[i].state == Finished_R){
            if(self->clients[i].recv_offset == self->clients[i].req_headers.size){
                self->clients[i].recv_offset = 0;
                self->clients[i].state = Available;
            }else{
                self->clients[i].state = Reading;
            }
            continue;
        }

        if(self->clients[i].state == Ready){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            sqe->user_data = OP_Write;
            self->clients[i].writev_offset = 0;
            self->clients[i].activity = now;
            self->clients[i].state = WrittingSock;
            REGISTER;
            continue;
        }
        if(self->clients[i].state == WrittingSock){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            sqe->user_data = OP_Write;
            io_uring_prep_write(sqe, self->clients[i].sock, (self->clients[i].response)+self->clients[i].writev_offset, self->clients[i].response_size,0);
            if(self->clients[i].writev_offset == self->clients[i].response_size){
                self->clients[i].state = Idle; 
            }
            REGISTER;
            continue;
        }
        if(self->clients[i].state == Kill){
            self->clients[i].state = NonExistent;
            self->clients[i].recv_offset = 0;
            self->clients[i].writev_offset = 0;
            free(self->clients[i].response);
            free(self->clients[i].request);
        }
    }
    {
        int res = io_uring_submit(&ring);
        if (res < 0){
            return res;
        };
    }

    for(usize i = 0; i < calls; i++){
        u64 ptr = self->author_log[i];
        struct io_uring_cqe *cqe;
        io_uring_wait_cqe(&ring, &cqe);
        __S32_TYPE res = cqe->res;

        if(res < 0){
            self->clients[ptr].state = Kill;
            continue;
        }

        int what = cqe->user_data;
        io_uring_cqe_seen(&ring, cqe);
        if(cqe->res < 0){
            continue;
        }
        
        if(what == OP_Read){
            self->clients[ptr].recv_offset += res;
            continue;
        }
        if(what == OP_Write){
            self->clients[ptr].writev_offset += res;
            continue;
        }
        if(what == OP_SocketAcc){
            self->clients[ptr].sock = res;
            continue;
        }
    }

    return 0;
}


//  RUST TOOLS 


int apply_client_response(Networker* self, u64 client_id, unsigned char* buffer, u64 buffer_size, int compression_algorithm){
    if (!(client_id < self->client_num-1 && self->clients[client_id].state == Processing)){
        return ENOPKG;
    }
    MessageHeaders headers;
    headers.size =  buffer_size;
    headers.compr_alg = compression_algorithm;
    usize rbs = sizeof(MessageHeaders) + buffer_size;
    unsigned char* response_buffer = malloc(rbs);
    if(!response_buffer){
        return ENOMEM;
    }
    memcpy(response_buffer, &headers, sizeof(MessageHeaders));
    memcpy(response_buffer+sizeof(MessageHeaders), buffer, buffer_size);
    self->clients[client_id].response = response_buffer;
    self->clients[client_id].response_size = rbs;
    self->clients[client_id].state = Ready;
    return 0;
}

typedef struct {
    Client* client;
    usize exists;
} SomeClient;  

SomeClient get_client(Networker* self){
    for(usize i = 0; i < self->client_num; i ++) {
        if(self->clients[i].state == Available){
            return (SomeClient) {&self->clients[i],1};
        }
    }
    return (SomeClient){NULL, 0};
}

int claim_client(Networker* self, u64 client_id){
    if(client_id < self->client_num && self->clients[client_id].state == Available){
        self->clients[client_id].state = Processing;
        return 0;
    }
    return ENOPKG;
}
int kill_client(Networker* self, u64 client_id){
    if(client_id < self->client_num){
        self->clients[client_id].state = Kill;
        return 0;
    }
    return ENOPKG;
}

int cycle(Networker* self){
    if (self->initiated != 1) return -1;
    return proc(self); // I may use another "proc" function with a non-io_uring clock, for compatibility reasons 
}
