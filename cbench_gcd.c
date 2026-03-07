// C reference: The Euclidean Measure (GCD)
// Mirrors samples/gcd_measure.onu
#include <stdio.h>

static long long gcd(long long a, long long b) {
    while (b != 0) {
        long long r = a % b;
        a = b;
        b = r;
    }
    return a;
}

int main(void) {
    printf("Common measure of 48 and 18:    %lld\n", gcd(48, 18));
    printf("Common measure of 252 and 105:  %lld\n", gcd(252, 105));
    printf("Common measure of 1071 and 462: %lld\n", gcd(1071, 462));
    printf("Common measure of 1M and (1M-1): %lld\n", gcd(1000000, 999999));
    return 0;
}
