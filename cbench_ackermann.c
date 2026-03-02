#include <stdio.h>

long long ackermann(long long depth, long long intensity) {
    if (depth == 0) return intensity + 1;
    if (intensity == 0) return ackermann(depth - 1, 1);
    return ackermann(depth - 1, ackermann(depth, intensity - 1));
}

int main() {
    printf("Ackermann(3, 11) Result: %lld\n", ackermann(3, 11));
    return 0;
}
