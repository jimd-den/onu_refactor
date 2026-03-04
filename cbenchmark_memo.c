#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define CACHE_SIZE 1000

long long cache[CACHE_SIZE];

long long fib_memo(long long n) {
  if (n <= 1)
    return n;
  if (cache[n] != -1)
    return cache[n];

  cache[n] = fib_memo(n - 1) + fib_memo(n - 2);
  return cache[n];
}

int main() {
  for (int i = 0; i < CACHE_SIZE; i++)
    cache[i] = -1;

  struct timespec start, end;
  clock_gettime(CLOCK_MONOTONIC, &start);

  long long result = fib_memo(40);

  clock_gettime(CLOCK_MONOTONIC, &end);

  double time_taken = (end.tv_sec - start.tv_sec) * 1e9;
  time_taken = (time_taken + (end.tv_nsec - start.tv_nsec)) * 1e-9;

  printf("Fib(40) = %lld\n", result);
  printf("Time: %f seconds\n", time_taken);

  return 0;
}
