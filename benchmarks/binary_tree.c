#include "bench_timing.h"

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

static long long run_benchmark(void) {
    bench_lcg_seed(12345);
    pool_idx = 0;

    Node *root = NULL;
    for (int i = 0; i < NUM_NODES; i++) {
        int key = (int)((bench_lcg_rand() << 15) | bench_lcg_rand());
        root = bst_insert(root, key);
    }

    return inorder_sum(root);
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    volatile long long sink;
    BENCH_TIME(niters, { sink = run_benchmark(); });

    return 0;
}
