#include <stdio.h>

long long sum_to(long long n, long long accumulator) {
  if (n == 0)
    return accumulator;
  return sum_to(n - 1, accumulator + n);
}

int main() {
  printf("Result: %lld\n", sum_to(1000000000, 0));
  return 0;
}
