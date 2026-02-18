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

/* --- Variant 1: Iterative tree height --- */

static int tree_height(Node *root) {
    if (!root) return 0;
    /* BFS with level tracking */
    Node **queue = (Node **)malloc(NUM_NODES * sizeof(Node *));
    int front = 0, back = 0, height = 0;
    queue[back++] = root;
    while (front < back) {
        int level_size = back - front;
        height++;
        int i;
        for (i = 0; i < level_size; i++) {
            Node *n = queue[front++];
            if (n->left) queue[back++] = n->left;
            if (n->right) queue[back++] = n->right;
        }
    }
    free(queue);
    return height;
}

/* --- Variant 2: BST validation via iterative inorder --- */

static int is_valid_bst(Node *root) {
    Node **stack = (Node **)malloc(NUM_NODES * sizeof(Node *));
    int top = 0;
    Node *cur = root;
    long long prev = -2147483649LL;
    int valid = 1;

    while ((cur || top > 0) && valid) {
        while (cur) {
            stack[top++] = cur;
            cur = cur->left;
        }
        cur = stack[--top];
        if (cur->key <= prev) { valid = 0; break; }
        prev = cur->key;
        cur = cur->right;
    }
    free(stack);
    return valid;
}

/* --- Variant 3: Count nodes in range with pruning --- */

static int count_in_range(Node *root, int lo, int hi) {
    Node **stack = (Node **)malloc(NUM_NODES * sizeof(Node *));
    int top = 0, count = 0;
    if (root) stack[top++] = root;

    while (top > 0) {
        Node *n = stack[--top];
        if (n->key >= lo && n->key <= hi) count++;
        if (n->left && n->key > lo) stack[top++] = n->left;
        if (n->right && n->key < hi) stack[top++] = n->right;
    }
    free(stack);
    return count;
}

static long long run_benchmark(void) {
    bench_lcg_seed(12345);
    pool_idx = 0;

    Node *root = NULL;
    for (int i = 0; i < NUM_NODES; i++) {
        int key = (int)((bench_lcg_rand() << 15) | bench_lcg_rand());
        root = bst_insert(root, key);
    }

    long long result = inorder_sum(root);
    result += tree_height(root);
    result += is_valid_bst(root);
    result += count_in_range(root, 100000, 500000);
    return result;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    volatile long long sink;
    BENCH_TIME(niters, { sink = run_benchmark(); });

    return 0;
}
