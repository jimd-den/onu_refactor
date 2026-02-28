#include <stdio.h>

long long fibonacci(long long n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

int main() {
    printf("%lld\n", fibonacci(40));
    return 0;
}
