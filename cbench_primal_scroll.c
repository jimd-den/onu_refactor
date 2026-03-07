// C reference: The Primal Scroll (Brainfuck interpreter)
// Mirrors samples/primal_scroll.onu — runs ++++++++[>++++++<-]>.
#include <stdio.h>
#include <string.h>

#define TAPE_SIZE 30
#define SCROLL_MAX 256

static int tape[TAPE_SIZE];
static char scroll[] = "++++++++[>++++++<-]>.";

static int scan_close(int ip) {
    int depth = 0;
    while (scroll[ip]) {
        if (scroll[ip] == '[') depth++;
        if (scroll[ip] == ']') { if (depth == 0) return ip; depth--; }
        ip++;
    }
    return ip;
}

static int scan_open(int ip) {
    int depth = 0;
    while (ip >= 0) {
        if (scroll[ip] == ']') depth++;
        if (scroll[ip] == '[') { if (depth == 0) return ip; depth--; }
        ip--;
    }
    return ip;
}

int main(void) {
    memset(tape, 0, sizeof(tape));
    int ip = 0, focus = 0;
    int len = (int)strlen(scroll);
    printf("Output of scroll (%s):\n", scroll);
    while (ip < len) {
        char sym = scroll[ip];
        if      (sym == '+') tape[focus]++;
        else if (sym == '-') tape[focus]--;
        else if (sym == '>') focus++;
        else if (sym == '<') focus--;
        else if (sym == '.') putchar(tape[focus]);
        else if (sym == '[' && tape[focus] == 0) ip = scan_close(ip + 1);
        else if (sym == ']' && tape[focus] != 0) ip = scan_open(ip - 1);
        ip++;
    }
    putchar('\n');
    return 0;
}
