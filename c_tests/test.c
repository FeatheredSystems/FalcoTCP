#include "../native/net.h"
#include <stdio.h>
#include <threads.h>
#include <errno.h>

int main(){

    Networker net = {0};  // Zero-initialize!
    struct NetworkerSettings config = {0};  // Zero-initialize!
    
    config.host[0] = '1';config.host[1] = '2';config.host[2] = '7';
    config.host[3] = '.';config.host[4] = '0';config.host[5] = '.';
    config.host[6] = '0';config.host[7] = '.';config.host[8] = '1';
    config.host[9] = '\0';

    config.max_clients = 10;
    config.max_queue = 10;
    config.port = 8080;

    printf("Starting server on 127.0.0.1:8080...\n");
    int bad = start(&net, &config);
    if (bad < 0){
        printf("FAILED to start server! Error code: %d (errno: %d)\n", bad, errno);
        return -1;
    }
    
    printf("Server started successfully! Socket fd: %d\n", net.sock);
    printf("Running cycles...\n");
    
    int cycles = 0;
    while(1){
        int result = cycle(&net);
        if(result < 0){
            printf("Cycle failed with error: %d\n", result);
            break;
        }
        cycles++;
        if(cycles % 10000 == 0){
            printf("Cycles: %d\n", cycles);
        }
        thrd_yield();
    }
    return 0;
}
