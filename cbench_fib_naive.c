// C naive-only fib for comparison
#include <stdint.h>
#include <stdio.h>

static long long fib(long long n) {
  if (n == 0)
    return 0;
  if (n == 1)
    return 1;
  return fib(n - 1) + fib(n - 2);
}

int main(void) {
  printf("fib(40) = %lld\n", fib(40));
  return 0;
}
