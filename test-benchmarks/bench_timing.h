/*
 * Shared benchmark timing harness.
 *
 * Each benchmark includes this header and calls BENCH_TIME() in main().
 * Iteration count is controlled via argv[1] (default 201), so the Rust
 * harness is the single source of truth for how many internal iterations
 * each benchmark binary runs.
 */
#ifndef BENCH_TIMING_H
#define BENCH_TIMING_H

#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>

static long long _bench_timespec_diff_ns(struct timespec *a, struct timespec *b) {
    return (long long)(b->tv_sec - a->tv_sec) * 1000000000LL + (b->tv_nsec - a->tv_nsec);
}

static int _bench_cmp_ll(const void *a, const void *b) {
    long long x = *(const long long *)a, y = *(const long long *)b;
    return (x > y) - (x < y);
}

static int bench_parse_iters(int argc, char **argv) {
    if (argc > 1) {
        int n = atoi(argv[1]);
        if (n >= 10) return n;
    }
    return 201;
}

/*
 * BENCH_TIME(niters, timed_expr)
 *
 * Warmup:  5 untimed executions of timed_expr.
 * Timing:  niters timed executions.
 * Output:  10%-trimmed mean in nanoseconds, printed to stdout.
 *
 * The caller must declare a volatile sink variable before invoking this
 * macro so the compiler cannot elide the workload.
 *
 * Example:
 *   volatile long long sink;
 *   BENCH_TIME(niters, { sink = workload(data); });
 */
#define BENCH_TIME(niters, timed_expr) do {                                   \
    /* Warmup */                                                              \
    for (int _w = 0; _w < 5; _w++) { timed_expr; }                           \
    /* Timed runs */                                                          \
    long long *_times = (long long *)malloc((niters) * sizeof(long long));    \
    struct timespec _t0, _t1;                                                 \
    for (int _i = 0; _i < (niters); _i++) {                                  \
        clock_gettime(CLOCK_MONOTONIC, &_t0);                                 \
        timed_expr;                                                           \
        clock_gettime(CLOCK_MONOTONIC, &_t1);                                 \
        _times[_i] = _bench_timespec_diff_ns(&_t0, &_t1);                    \
    }                                                                         \
    qsort(_times, (niters), sizeof(long long), _bench_cmp_ll);               \
    int _trim = (niters) / 10;                                                \
    int _count = (niters) - 2 * _trim;                                        \
    long long _tsum = 0;                                                      \
    for (int _ti = _trim; _ti < (niters) - _trim; _ti++) _tsum += _times[_ti];\
    printf("%lld\n", _tsum / _count);                                         \
    free(_times);                                                             \
} while(0)

/* LCG PRNG — shared across all benchmarks for reproducible data */
static unsigned int _bench_lcg_state = 12345;
static unsigned int bench_lcg_rand(void) {
    _bench_lcg_state = _bench_lcg_state * 1103515245 + 12345;
    return (_bench_lcg_state >> 16) & 0x7fff;
}
static void bench_lcg_seed(unsigned int seed) {
    _bench_lcg_state = seed;
}

#endif /* BENCH_TIMING_H */
