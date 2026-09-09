#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define MAX_NAME_LENGTH 100
#define BUFFER_SIZE 1024


/* Constants for configuration limits */
#define MIN_TIMEOUT 1000
#define MAX_TIMEOUT 10000
#define MAX_RETRIES 5

/**
 * HelloWorld structure represents a greeter object with configuration
 * Contains:
 * - name: String identifier for the greeter (max 100 chars)
 * - created_at: Timestamp when instance was created
 * - timeout: Milliseconds to wait between greetings (1000-10000)
 * - retries: Number of retry attempts (0-5)
 */
typedef struct {
    char name[MAX_NAME_LENGTH];
    time_t created_at;
    int timeout;
    int retries;
} HelloWorld;

HelloWorld* hello_world_create(const char* name) {
    HelloWorld* hw = (HelloWorld*)malloc(sizeof(HelloWorld));
    if (!hw) return NULL;
    
    strncpy(hw->name, name, MAX_NAME_LENGTH - 1);
    hw->name[MAX_NAME_LENGTH - 1] = '\0';
    hw->created_at = time(NULL);
    hw->timeout = 5000;
    hw->retries = 3;
    
    return hw;
}

void hello_world_destroy(HelloWorld* hw) {
    if (hw) {
        free(hw);
    }
}

void hello_world_greet(HelloWorld* hw, const char** names, int count) {
    for (int i = 0; i < count; i++) {
        printf("Hello, %s from %s!\n", names[i], hw->name);
    }
}

void hello_world_configure(HelloWorld* hw, int timeout, int retries) {
    hw->timeout = timeout;
    hw->retries = retries;
}

char* hello_world_generate_report(const HelloWorld* hw) {
    char* report = (char*)malloc(BUFFER_SIZE);
    char time_str[26];
    ctime_r(&hw->created_at, time_str);
    time_str[24] = '\0';
    
    snprintf(report, BUFFER_SIZE,
        "HelloWorld Report\n"
        "================\n"
        "Name: %s\n"
        "Created: %s\n"
        "Timeout: %d\n"
        "Retries: %d\n",
        hw->name, time_str, hw->timeout, hw->retries);
    
    return report;
}

int main() {
    HelloWorld* greeter = hello_world_create("C Example");
    
    const char* names[] = {"Alice", "Bob"};
    int names_count = sizeof(names) / sizeof(names[0]);
    
    hello_world_configure(greeter, 1000, 5);
    hello_world_greet(greeter, names, names_count);
    
    char* report = hello_world_generate_report(greeter);
    printf("%s\n", report);
    free(report);
    
    hello_world_destroy(greeter);
    return 0;
}
