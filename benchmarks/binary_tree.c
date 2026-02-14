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
static unsigned int lcg_rand(void) { lcg_state = lcg_state * 1103515245 + 12345; return (lcg_state >> 16) & 0x7fff; }

#define NUM_NODES 2000

typedef struct Node {
    int key;
    struct Node *left;
    struct Node *right;
} Node;

static Node pool[NUM_NODES];
static int pool_idx;

static Node *node_alloc(int key) {
    Node *n = &pool[pool_idx++];
    n->key = key;
    n->left = NULL;
    n->right = NULL;
    return n;
}

static Node *bst_insert(Node *root, int key) {
    if (!root) return node_alloc(key);
    Node *cur = root;
    for (;;) {
        if (key < cur->key) {
            if (!cur->left) { cur->left = node_alloc(key); return root; }
            cur = cur->left;
        } else if (key > cur->key) {
            if (!cur->right) { cur->right = node_alloc(key); return root; }
            cur = cur->right;
        } else {
            return root; /* duplicate */
        }
    }
}

/* Iterative inorder traversal using explicit stack */
static long long inorder_sum(Node *root) {
    long long sum = 0;
    Node **stack = (Node **)malloc(NUM_NODES * sizeof(Node *));
    int top = 0;
    Node *cur = root;

    while (cur || top > 0) {
        while (cur) {
            stack[top++] = cur;
            cur = cur->left;
        }
        cur = stack[--top];
        sum += cur->key;
        cur = cur->right;
    }

    free(stack);
    return sum;
}

static volatile long long sink;

static void run_benchmark(void) {
    lcg_state = 12345;
    pool_idx = 0;

    Node *root = NULL;
    for (int i = 0; i < NUM_NODES; i++) {
        int key = (int)((lcg_rand() << 15) | lcg_rand());
        root = bst_insert(root, key);
    }

    sink = inorder_sum(root);
}

int main(void) {
    for (int i = 0; i < 5; i++) run_benchmark();

    long long times[50];
    for (int i = 0; i < 50; i++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        run_benchmark();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[i] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);
    return 0;
}
