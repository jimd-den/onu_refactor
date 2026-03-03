// C reference implementation for fib_bench comparison
// Compiled with: gcc -O3 -foptimize-sibling-calls -o cbench_fib cbench_fib.c
// Mirrors the three Onu variants exactly.

#include <stdint.h>
#include <stdio.h>

// Variant 1: Naive double recursion — matches fib-naive in Onu
static long long fib_naive(long long n) {
  if (n == 0)
    return 0;
  if (n == 1)
    return 1;
  return fib_naive(n - 1) + fib_naive(n - 2);
}

// Variant 2: Tail-recursive accumulator — matches fib-tco in Onu
static long long fib_tco(long long n, long long a, long long b) {
  if (n == 0)
    return a;
  return fib_tco(n - 1, b, a + b);
}

// Variant 3: Sum of fib_tco(0..target) — matches fib-sum-range in Onu
static long long fib_sum_range(long long current, long long target,
                               long long acc) {
  if (current > target)
    return acc;
  long long fib_val = fib_tco(current, 0, 1);
  return fib_sum_range(current + 1, target, acc + fib_val);
}

int main(void) {
  long long naive_result = fib_naive(100);
  long long tco_result = fib_tco(10000, 0, 1);
  long long range_result = fib_sum_range(0, 92, 0);

  printf("fib-naive(40) = %lld\n", naive_result);
  printf("fib-tco(10000) mod 1000000 = %lld\n", tco_result % 1000000);
  printf("sum(fib(0..92)) = %lld\n", range_result);

  return 0;
}
