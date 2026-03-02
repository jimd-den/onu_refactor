#include <stdio.h>
#include <string.h>
#include <stdint.h>

typedef struct {
    char key[32];
    int64_t value;
    int occupied;
} MapSlot;

typedef struct {
    MapSlot slots[10];
} HashMap;

uint64_t calculate_hash(const char* label) {
    return strlen(label) % 10;
}

void put(HashMap* map, const char* label, int64_t val) {
    uint64_t idx = calculate_hash(label);
    strncpy(map->slots[idx].key, label, 31);
    map->slots[idx].value = val;
    map->slots[idx].occupied = 1;
}

int64_t get(HashMap* map, const char* label) {
    uint64_t idx = calculate_hash(label);
    if (map->slots[idx].occupied && strcmp(map->slots[idx].key, label) == 0) {
        return map->slots[idx].value;
    }
    return 0;
}

int main() {
    HashMap map = {0};
    
    // Benchmark loop: 100,000 iterations
    for (int i = 0; i < 100000; i++) {
        put(&map, "Alpha", 100);
        put(&map, "Beta", 200);
        put(&map, "Gamma", 300);
        
        get(&map, "Alpha");
        get(&map, "Beta");
        get(&map, "Gamma");
    }
    
    printf("Alpha: %ld\n", get(&map, "Alpha"));
    printf("Beta: %ld\n", get(&map, "Beta"));
    printf("Gamma: %ld\n", get(&map, "Gamma"));
    
    return 0;
}
