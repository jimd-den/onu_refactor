// C reference: The Balancing Act (Tak function)
// Mirrors samples/tak_balance.onu
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Simple memoization with a hash map for tak(x,y,z)
#define MEMO_SIZE (1 << 19)
#define HASH_PRIME 2654435769ULL

typedef struct {
    long long x, y, z, result;
    int valid;
} TakEntry;

static TakEntry memo[MEMO_SIZE];

static long long tak_key(long long x, long long y, long long z) {
    unsigned long long h = (unsigned long long)x * HASH_PRIME
                         + (unsigned long long)y * HASH_PRIME * HASH_PRIME
                         + (unsigned long long)z * HASH_PRIME * HASH_PRIME * HASH_PRIME;
    return (long long)(h & (MEMO_SIZE - 1));
}

static long long tak(long long x, long long y, long long z) {
    if (x <= y) return z;
    long long slot = tak_key(x, y, z);
    if (memo[slot].valid && memo[slot].x == x && memo[slot].y == y && memo[slot].z == z)
        return memo[slot].result;
    long long r = tak(tak(x-1, y, z), tak(y-1, z, x), tak(z-1, x, y));
    memo[slot].x = x; memo[slot].y = y; memo[slot].z = z;
    memo[slot].result = r; memo[slot].valid = 1;
    return r;
}

int main(void) {
    memset(memo, 0, sizeof(memo));
    printf("Balance(18, 12, 6) = %lld\n", tak(18, 12, 6));
    return 0;
}
