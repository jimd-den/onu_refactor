// C reference: The Tower of Balance (Hanoi)
// Mirrors samples/hanoi.onu
#include <stdio.h>

static int moves = 0;

static void hanoi(int n, char from, char to, char via) {
    if (n == 1) {
        printf("Move disk 1 from %c to %c\n", from, to);
        moves++;
        return;
    }
    hanoi(n - 1, from, via, to);
    printf("Move disk %d from %c to %c\n", n, from, to);
    moves++;
    hanoi(n - 1, via, to, from);
}

int main(void) {
    moves = 0;
    hanoi(12, 'A', 'C', 'B');
    printf("Total moves for 12 disks: %d\n", moves);
    return 0;
}
