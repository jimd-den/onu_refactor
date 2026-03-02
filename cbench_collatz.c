#include <stdio.h>

long long collatz_steps(long long n, long long count) {
    if (n == 1) return count;
    if (n % 2 == 0) return collatz_steps(n / 2, count + 1);
    return collatz_steps(n * 3 + 1, count + 1);
}

long long collatz_range(long long current, long long target, long long accumulator) {
    if (current > target) return accumulator;
    return collatz_range(current + 1, target, accumulator + collatz_steps(current, 0));
}

int main() {
    long long limit = 1000000;
    printf("Total Collatz steps for 1 to %lld is: %lld\n", limit, collatz_range(1, limit, 0));
    return 0;
}
