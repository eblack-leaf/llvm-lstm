#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>

static long long timespec_diff_ns(struct timespec *a, struct timespec *b) {
    return (long long)(b->tv_sec - a->tv_sec) * 1000000000LL + (b->tv_nsec - a->tv_nsec);
}

static int cmp_ll(const void *a, const void *b) {
    long long x = *(const long long *)a, y = *(const long long *)b;
    return (x > y) - (x < y);
}

static unsigned int lcg_state = 12345;
static unsigned int lcg_rand(void) {
    lcg_state = lcg_state * 1103515245 + 12345;
    return (lcg_state >> 16) & 0x7fff;
}

#define NUM_OPS    3000
#define TABLE_SIZE 6007    /* prime, ~2x NUM_OPS */

#define SLOT_EMPTY    0
#define SLOT_OCCUPIED 1
#define SLOT_DELETED  2

typedef struct {
    unsigned int key;
    unsigned int value;
    unsigned char state;
} Slot;

/* --- Multiple hash functions: instcombine targets on bitwise mixing --- */

static unsigned int hash_mul(unsigned int key) {
    /* Multiplicative hash (Knuth) */
    return (key * 2654435761u) >> 16;
}

static unsigned int hash_jenkins(unsigned int key) {
    /* Jenkins one-at-a-time */
    unsigned int h = 0;
    h += key & 0xFF; h += h << 10; h ^= h >> 6;
    h += (key >> 8) & 0xFF; h += h << 10; h ^= h >> 6;
    h += (key >> 16) & 0xFF; h += h << 10; h ^= h >> 6;
    h += (key >> 24) & 0xFF; h += h << 10; h ^= h >> 6;
    h += h << 3; h ^= h >> 11; h += h << 15;
    return h;
}

static unsigned int hash_murmur_mix(unsigned int key) {
    /* Murmur3 finalizer */
    key ^= key >> 16;
    key *= 0x85ebca6bu;
    key ^= key >> 13;
    key *= 0xc2b2ae35u;
    key ^= key >> 16;
    return key;
}

/* --- Linear probing table --- */

static Slot table_lin[TABLE_SIZE];

static void lin_clear(void) { memset(table_lin, 0, sizeof(table_lin)); }

static void lin_insert(unsigned int key, unsigned int value) {
    unsigned int idx = hash_mul(key) % TABLE_SIZE;
    while (table_lin[idx].state == SLOT_OCCUPIED) {
        if (table_lin[idx].key == key) {
            table_lin[idx].value = value;
            return;
        }
        idx = (idx + 1) % TABLE_SIZE;
    }
    table_lin[idx].key = key;
    table_lin[idx].value = value;
    table_lin[idx].state = SLOT_OCCUPIED;
}

static unsigned int lin_lookup(unsigned int key) {
    unsigned int idx = hash_mul(key) % TABLE_SIZE;
    while (table_lin[idx].state != SLOT_EMPTY) {
        if (table_lin[idx].state == SLOT_OCCUPIED && table_lin[idx].key == key)
            return table_lin[idx].value;
        idx = (idx + 1) % TABLE_SIZE;
    }
    return 0;
}

static void lin_delete(unsigned int key) {
    unsigned int idx = hash_mul(key) % TABLE_SIZE;
    while (table_lin[idx].state != SLOT_EMPTY) {
        if (table_lin[idx].state == SLOT_OCCUPIED && table_lin[idx].key == key) {
            table_lin[idx].state = SLOT_DELETED;
            return;
        }
        idx = (idx + 1) % TABLE_SIZE;
    }
}

/* --- Quadratic probing table (different probing = different branch pattern) --- */

static Slot table_quad[TABLE_SIZE];

static void quad_clear(void) { memset(table_quad, 0, sizeof(table_quad)); }

static void quad_insert(unsigned int key, unsigned int value) {
    unsigned int base = hash_jenkins(key) % TABLE_SIZE;
    unsigned int i;
    for (i = 0; i < TABLE_SIZE; i++) {
        unsigned int idx = (base + i * i) % TABLE_SIZE;
        if (table_quad[idx].state != SLOT_OCCUPIED) {
            table_quad[idx].key = key;
            table_quad[idx].value = value;
            table_quad[idx].state = SLOT_OCCUPIED;
            return;
        }
        if (table_quad[idx].key == key) {
            table_quad[idx].value = value;
            return;
        }
    }
}

static unsigned int quad_lookup(unsigned int key) {
    unsigned int base = hash_jenkins(key) % TABLE_SIZE;
    unsigned int i;
    for (i = 0; i < TABLE_SIZE; i++) {
        unsigned int idx = (base + i * i) % TABLE_SIZE;
        if (table_quad[idx].state == SLOT_EMPTY) return 0;
        if (table_quad[idx].state == SLOT_OCCUPIED && table_quad[idx].key == key)
            return table_quad[idx].value;
    }
    return 0;
}

/* --- Main workload --- */

static unsigned int keys[NUM_OPS];
static unsigned int values[NUM_OPS];

static volatile unsigned long long sink;

static void run_benchmark(void) {
    lcg_state = 12345;
    int i;

    for (i = 0; i < NUM_OPS; i++) {
        keys[i] = hash_murmur_mix((lcg_rand() << 15) | lcg_rand());
        values[i] = (lcg_rand() << 15) | lcg_rand();
    }

    unsigned long long sum = 0;

    /* Phase 1: linear probing — insert, lookup, delete, re-lookup */
    lin_clear();
    for (i = 0; i < NUM_OPS; i++) lin_insert(keys[i], values[i]);
    for (i = 0; i < NUM_OPS; i++) sum += lin_lookup(keys[i]);

    /* Delete every 3rd key, then lookup all (mix of hits and misses) */
    for (i = 0; i < NUM_OPS; i += 3) lin_delete(keys[i]);
    for (i = 0; i < NUM_OPS; i++) sum += lin_lookup(keys[i]);

    /* Phase 2: quadratic probing — same keys, different structure */
    quad_clear();
    for (i = 0; i < NUM_OPS; i++) quad_insert(keys[i], values[i]);
    for (i = 0; i < NUM_OPS; i++) sum += quad_lookup(keys[i]);

    sink = sum;
}

int main(void) {
    int i;
    for (i = 0; i < 5; i++) run_benchmark();

    long long times[201];
    for (i = 0; i < 201; i++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        run_benchmark();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[i] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);
    return 0;
}
