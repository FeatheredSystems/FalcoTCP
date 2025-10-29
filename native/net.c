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
#include "numbers.h"
#include <liburing.h>
#include "net.h"

#define sfree(p) do { free(p); (p) = NULL; } while(0)

#define MESSAGE_HEADERS_SIZE 9






// Serialize into a buffer (little-endian)
static inline void serialize_message_headers(const MessageHeaders *msg, uint8_t *buf) {
    // store size in little-endian
    for (int i = 0; i < 8; i++) {
        buf[i] = (msg->size >> (i * 8)) & 0xFF;
    }
    buf[8] = msg->compr_alg;  // 1 byte
}

// Deserialize from a buffer (little-endian)
static inline void deserialize_message_headers(const uint8_t *buf, MessageHeaders *msg) {
    msg->size = 0;
    for (int i = 0; i < 8; i++) {
        msg->size |= ((uint64_t)buf[i]) << (i * 8);
    }
    msg->compr_alg = buf[8];
}
int start(Networker* self, struct NetworkerSettings* s){
    struct NetworkerSettings settings = *s;
    if (self->initiated > 0){
        return 0;
    }
    self->client_num = s->max_clients;
    int sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock < 0){
        return -errno;
    }
    
    struct  sockaddr_in sockad;
    sockad.sin_family = AF_INET;
    sockad.sin_port = htons(settings.port);
    inet_pton(AF_INET, settings.host, &sockad.sin_addr);
    int _l = bind(sock, (struct sockaddr*)&sockad, sizeof(sockad));
    if (_l < 0){
        return -errno;
    }

    int _l1 = listen(sock, settings.max_queue);
    if (_l1 < 0){
        return -errno;
    }
    self->sock = sock;
    self->initiated = 1;

    self->clients = (Client*)calloc(self->client_num, sizeof(Client));
    if(!self->clients){
        return -ENOMEM;  
    }
    for(usize i = 0; i < self->client_num; i++){
        self->clients[i].id = i;
        self->clients[i].request = NULL;
        self->clients[i].response = NULL;
        self->clients[i].state = NonExistent;
    }

    self->author_log = calloc(self->client_num,sizeof(u64));
    if(!self->author_log){
        free(self->clients);
        close(sock);
        return -ENOMEM;
    }
    self->ring = calloc(1,sizeof(struct io_uring));
    if (!self->ring){
        free(self->author_log);
        free(self->clients);
        close(sock);
        return -ENOMEM;
    }
    {
        int res = io_uring_queue_init(self->client_num>0?self->client_num:1, self->ring, 0);
        if(res < 0) {
            free(self->author_log);
            free(self->clients);
            free(self->ring);
            close(sock);
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
    
    for(u64 i = 0; i < self->client_num; i++){
        if(self->clients[i].state == NonExistent){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            io_uring_prep_accept(sqe, self->sock, NULL,NULL, 0);
            sqe->user_data = OP_SocketAcc;
            REGISTER; 
            continue;
        }        
        if(self->clients[i].state == Idle){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            if((now-self->clients[i].activity) > 1200){
                sqe->user_data = OP_Close;
                io_uring_prep_close(sqe, self->clients[i].sock);
                self->clients[i].state = NonExistent;
                sfree(self->clients[i].response);
                sfree(self->clients[i].request);
                continue;
            }
            sqe->user_data = OP_Read;
            self->clients[i].state = Finished_H;
            io_uring_prep_read(sqe,self->clients[i].sock, (char*)(&self->clients[i].req_headers)+self->clients[i].recv_offset, MESSAGE_HEADERS_SIZE-self->clients[i].recv_offset, 0);
            REGISTER;
            continue;
        }

        if(self->clients[i].state == Finished_H){
            if(self->clients[i].recv_offset == MESSAGE_HEADERS_SIZE){
                self->clients[i].recv_offset = 0;
                unsigned char buffer [MESSAGE_HEADERS_SIZE];
                memcpy(buffer, &self->clients[i].req_headers, MESSAGE_HEADERS_SIZE);
                deserialize_message_headers(buffer, &self->clients[i].req_headers);
                self->clients[i].state = Reading;
            }else{
                self->clients[i].state = Idle;
                continue;
            }
        }

        if(self->clients[i].state == Reading){
            if(self->clients[i].capacity < self->clients[i].req_headers.size || self->clients[i].request == NULL){
                sfree(self->clients[i].request);
                self->clients[i].request = malloc(self->clients[i].req_headers.size);
                self->clients[i].capacity = self->clients[i].req_headers.size;
            }
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            sqe->user_data = OP_Read;
            self->clients[i].state = Finished_R;
            io_uring_prep_read(sqe,self->clients[i].sock,self->clients[i].request + self->clients[i].recv_offset,self->clients[i].req_headers.size-self->clients[i].recv_offset,0);
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
            self->clients[i].writev_offset = 0;
            self->clients[i].activity = now;
            self->clients[i].state = WrittingSock;
            continue;
        }
        if(self->clients[i].state == WrittingSock){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            sqe->user_data = OP_Write;
            io_uring_prep_write(sqe, self->clients[i].sock, 
                       (self->clients[i].response) + self->clients[i].writev_offset, 
                       self->clients[i].response_size - self->clients[i].writev_offset, 0);
            self->clients[i].state = Finished_WS;
            REGISTER;
            continue;
        }
        if(self->clients[i].state == Finished_WS){
            if(self->clients[i].writev_offset >= self->clients[i].response_size){
                self->clients[i].writev_offset = 0;
                self->clients[i].state = Idle;
            }else{
                self->clients[i].state = WrittingSock;
            }
            continue;
        }
        if(self->clients[i].state == Kill){
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            sqe->user_data = OP_Close;
            io_uring_prep_close(sqe, self->clients[i].sock);
            self->clients[i].state = NonExistent;
            self->clients[i].recv_offset = 0;
            self->clients[i].writev_offset = 0;
            sfree(self->clients[i].response);
            sfree(self->clients[i].request);
            REGISTER;
            continue;
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
        io_uring_cqe_seen(&ring, cqe);
        if(res < 0){
            self->clients[ptr].state = Kill;
            continue;
        }

        int what = cqe->user_data;
        
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
            u64 saved_id = self->clients[ptr].id;
            memset(&self->clients[ptr], 0, sizeof(Client));
            self->clients[ptr].sock = res;
            self->clients[ptr].state = Idle;
            self->clients[ptr].activity = now;
            self->clients[ptr].id = saved_id;
            continue;
        }
    }

    return 0;
}


//  RUST TOOLS 


int apply_client_response(Networker* self, u64 client_id, unsigned char* buffer, u64 buffer_size, int compression_algorithm){
    if (!(client_id < self->client_num && self->clients[client_id].state == Processing)){
        return -ENOPKG;
    }
    MessageHeaders headers;
    headers.size =  buffer_size;
    headers.compr_alg = compression_algorithm;
    usize rbs = sizeof(MessageHeaders) + buffer_size;
    unsigned char* response_buffer = malloc(rbs);
    if(!response_buffer){
        return -ENOMEM;
    }
    serialize_message_headers(&headers, response_buffer);
    memcpy(response_buffer+MESSAGE_HEADERS_SIZE, buffer, buffer_size);
    self->clients[client_id].response = response_buffer;
    self->clients[client_id].response_size = rbs;
    self->clients[client_id].state = Ready;
    return 0;
}
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
    return -ENOPKG;
}
int kill_client(Networker* self, u64 client_id){
    if(client_id < self->client_num){
        self->clients[client_id].state = Kill;
        return 0;
    }
    return -ENOPKG;
}

int cycle(Networker* self){
    if (self->initiated != 1) return -1;
    return proc(self); // I may use another "proc" function with a non-io_uring clock, for compatibility reasons 
}
