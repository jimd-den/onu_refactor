// C reference: The Simple Universe (Rule 110 cellular automaton)
// Mirrors samples/rule110.onu
#include <stdio.h>
#include <string.h>

#define WIDTH 16
#define STEPS 15

// Rule 110: binary 01101110. Patterns 0,4,7 -> 0; all others -> 1.
static int rule110(int left, int center, int right) {
    int pattern = left * 4 + center * 2 + right;
    return (pattern == 0 || pattern == 4 || pattern == 7) ? 0 : 1;
}

int main(void) {
    char gen[WIDTH + 1];
    char next[WIDTH + 1];
    memset(gen, '0', WIDTH);
    gen[WIDTH - 1] = '1';
    gen[WIDTH] = '\0';

    for (int step = 0; step <= STEPS; step++) {
        printf("%s\n", gen);
        for (int i = 0; i < WIDTH; i++) {
            int l = (i > 0)        ? gen[i-1] - '0' : 0;
            int c = gen[i] - '0';
            int r = (i < WIDTH-1)  ? gen[i+1] - '0' : 0;
            next[i] = '0' + rule110(l, c, r);
        }
        next[WIDTH] = '\0';
        memcpy(gen, next, WIDTH + 1);
    }
    return 0;
}
