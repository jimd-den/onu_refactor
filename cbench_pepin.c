// C reference: The Pépin Test (Fermat number primality)
// Mirrors samples/pepin_test.onu — uses __int128 for wide arithmetic.
#include <stdio.h>

typedef unsigned __int128 u128;

// Modular exponentiation: base^exp mod m
static u128 powmod(u128 base, u128 exp, u128 m) {
    u128 result = 1;
    base %= m;
    while (exp > 0) {
        if (exp & 1) result = result * base % m;
        base = base * base % m;
        exp >>= 1;
    }
    return result;
}

// Pépin's test: F_n = 2^(2^n) + 1 is prime iff 3^((F_n-1)/2) ≡ -1 (mod F_n)
static int pepin(int n) {
    // Compute F_n = 2^(2^n) + 1 using u128 (works for n <= 4)
    u128 fn = 1;
    for (int i = 0; i < (1 << n); i++) fn *= 2;
    fn += 1;
    u128 exponent = (fn - 1) / 2;
    u128 result = powmod(3, exponent, fn);
    return (result == fn - 1) ? 1 : 0;
}

int main(void) {
    for (int n = 1; n <= 4; n++) {
        // Compute F_n for display
        unsigned long long fn = 1;
        for (int i = 0; i < (1 << n); i++) fn *= 2;
        fn += 1;
        printf("F(%d) = %llu  -> prime? %d\n", n, fn, pepin(n));
    }
    return 0;
}
