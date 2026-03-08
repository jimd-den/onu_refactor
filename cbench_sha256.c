/*
 * SHA-256 benchmark: C reference matching samples/sha256.onu
 *
 * Same 1000 inputs: 32-bit LCG seeded at 42
 *   next(s) = (s * 1664525 + 1013904223) & 0xFFFFFFFF
 * Same message layout per block:
 *   W[0]  = lcg_value  (the "message")
 *   W[1]  = 0x80000000 (SHA-256 padding byte 0x80 in most-significant byte)
 *   W[2..14] = 0
 *   W[15] = 32         (message length in bits)
 *
 * Pure C, no libc crypto, no openssl.
 */

#include <stdint.h>
#include <stdio.h>

/* --- SHA-256 round constants ------------------------------------------- */
static const uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

/* --- SHA-256 initial hash values --------------------------------------- */
static const uint32_t H0[8] = {
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
};

#define ROTR32(x,n) (((x) >> (n)) | ((x) << (32-(n))))
#define SHR(x,n)    ((x) >> (n))
#define CH(e,f,g)   (((e)&(f)) ^ (~(e)&(g)))
#define MAJ(a,b,c)  (((a)&(b)) ^ ((a)&(c)) ^ ((b)&(c)))
#define SIGMA0(x)   (ROTR32(x,2)  ^ ROTR32(x,13) ^ ROTR32(x,22))
#define SIGMA1(x)   (ROTR32(x,6)  ^ ROTR32(x,11) ^ ROTR32(x,25))
#define sigma0(x)   (ROTR32(x,7)  ^ ROTR32(x,18) ^ SHR(x,3))
#define sigma1(x)   (ROTR32(x,17) ^ ROTR32(x,19) ^ SHR(x,10))

/* Hash one 512-bit block whose 16 words are given in W[0..15].          */
static void sha256_block(const uint32_t msg[16], uint32_t digest[8])
{
    uint32_t W[64];
    uint32_t a, b, c, d, e, f, g, h, T1, T2;
    int t;

    /* Prepare message schedule */
    for (t = 0; t < 16; t++)  W[t] = msg[t];
    for (t = 16; t < 64; t++)
        W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16];

    /* Initialise working variables */
    a = H0[0]; b = H0[1]; c = H0[2]; d = H0[3];
    e = H0[4]; f = H0[5]; g = H0[6]; h = H0[7];

    /* 64 compression rounds */
    for (t = 0; t < 64; t++) {
        T1 = h + SIGMA1(e) + CH(e,f,g) + K[t] + W[t];
        T2 = SIGMA0(a) + MAJ(a,b,c);
        h = g; g = f; f = e; e = d + T1;
        d = c; c = b; b = a; a = T1 + T2;
    }

    /* Davies-Meyer add-back */
    digest[0] = H0[0] + a; digest[1] = H0[1] + b;
    digest[2] = H0[2] + c; digest[3] = H0[3] + d;
    digest[4] = H0[4] + e; digest[5] = H0[5] + f;
    digest[6] = H0[6] + g; digest[7] = H0[7] + h;
}

int main(void)
{
    uint32_t seed = 42;
    uint32_t msg[16];
    uint32_t digest[8];
    int i;

    printf("SHA-256 Benchmark (1000 hashes, pure C):\n");
    for (i = 0; i < 1000; i++) {
        /* Advance LCG */
        seed = (uint32_t)((seed * 1664525ULL + 1013904223ULL) & 0xFFFFFFFFULL);
        /* Build message block */
        msg[0]  = seed;
        msg[1]  = 0x80000000u;
        for (int j = 2; j < 15; j++) msg[j] = 0;
        msg[15] = 32;
        sha256_block(msg, digest);
        /* Print hex digest */
        for (int j = 0; j < 8; j++)
            printf("%08x", digest[j]);
        printf("\n");
    }
    return 0;
}
