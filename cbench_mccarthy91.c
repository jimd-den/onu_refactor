// C reference: The McCarthy 91 Ritual
// Mirrors samples/mccarthy91.onu
#include <stdio.h>

static long long mc91(long long n) {
    if (n > 100) return n - 10;
    return mc91(mc91(n + 11));
}

int main(void) {
    printf("Ritual(0)   = %lld\n", mc91(0));
    printf("Ritual(50)  = %lld\n", mc91(50));
    printf("Ritual(99)  = %lld\n", mc91(99));
    printf("Ritual(100) = %lld\n", mc91(100));
    printf("Ritual(101) = %lld\n", mc91(101));
    printf("Ritual(150) = %lld\n", mc91(150));
    return 0;
}
