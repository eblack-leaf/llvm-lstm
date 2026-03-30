/*
 * Targets: jump-threading (dispatch loop), simplifycfg (switch elimination),
 * early-cse (repeated stack accesses), inline (helper functions),
 * mem2reg (stack/frame locals).
 *
 * Simple bytecode interpreter for a stack machine.
 */
#include "bench_timing.h"

#define STACK_SIZE 256
#define CODE_SIZE  2000
#define NUM_PROGRAMS 5

/* Opcodes */
enum {
    OP_PUSH = 0,   /* push immediate (next byte) */
    OP_POP,        /* pop and discard */
    OP_DUP,        /* duplicate top */
    OP_SWAP,       /* swap top two */
    OP_ADD,        /* pop two, push sum */
    OP_SUB,        /* pop two, push difference */
    OP_MUL,        /* pop two, push product */
    OP_DIV,        /* pop two, push quotient (no div by zero) */
    OP_MOD,        /* pop two, push modulo */
    OP_NEG,        /* negate top */
    OP_AND,        /* bitwise AND */
    OP_OR,         /* bitwise OR */
    OP_XOR,        /* bitwise XOR */
    OP_NOT,        /* bitwise NOT */
    OP_SHL,        /* shift left */
    OP_SHR,        /* shift right */
    OP_CMP_EQ,    /* compare equal: push 1 or 0 */
    OP_CMP_LT,    /* compare less: push 1 or 0 */
    OP_CMP_GT,    /* compare greater: push 1 or 0 */
    OP_JMP,        /* unconditional jump (next byte = offset) */
    OP_JZ,         /* jump if zero */
    OP_JNZ,        /* jump if not zero */
    OP_LOAD,       /* load from memory slot (next byte = slot) */
    OP_STORE,      /* store to memory slot (next byte = slot) */
    OP_CALL,       /* push return address, jump to offset */
    OP_RET,        /* pop return address, jump back */
    OP_INC,        /* increment top */
    OP_DEC,        /* decrement top */
    OP_OVER,       /* copy second-from-top to top */
    OP_ROT,        /* rotate top three: a b c -> b c a */
    OP_NOP,        /* no operation */
    OP_HALT,       /* stop execution */
    NUM_OPS
};

typedef struct {
    int stack[STACK_SIZE];
    int sp;              /* stack pointer */
    int memory[64];      /* memory slots */
    int call_stack[32];  /* return addresses */
    int csp;             /* call stack pointer */
} VM;

static void vm_init(VM *vm) {
    vm->sp = 0;
    vm->csp = 0;
    memset(vm->memory, 0, sizeof(vm->memory));
}

static void vm_push(VM *vm, int val) {
    if (vm->sp < STACK_SIZE) {
        vm->stack[vm->sp++] = val;
    }
}

static int vm_pop(VM *vm) {
    if (vm->sp > 0) {
        return vm->stack[--vm->sp];
    }
    return 0;
}

static int vm_peek(VM *vm) {
    if (vm->sp > 0) return vm->stack[vm->sp - 1];
    return 0;
}

/* Execute bytecode, return final top-of-stack */
static int vm_execute(VM *vm, const unsigned char *code, int codelen) {
    int ip = 0;
    int steps = 0;
    int max_steps = codelen * 4; /* prevent infinite loops */

    while (ip < codelen && steps < max_steps) {
        unsigned char op = code[ip++];
        steps++;

        switch (op % NUM_OPS) {
        case OP_PUSH:
            if (ip < codelen) {
                vm_push(vm, (int)code[ip++]);
            }
            break;
        case OP_POP:
            vm_pop(vm);
            break;
        case OP_DUP:
            vm_push(vm, vm_peek(vm));
            break;
        case OP_SWAP: {
            if (vm->sp >= 2) {
                int tmp = vm->stack[vm->sp - 1];
                vm->stack[vm->sp - 1] = vm->stack[vm->sp - 2];
                vm->stack[vm->sp - 2] = tmp;
            }
            break;
        }
        case OP_ADD: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a + b);
            break;
        }
        case OP_SUB: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a - b);
            break;
        }
        case OP_MUL: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a * b);
            break;
        }
        case OP_DIV: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, (b != 0) ? a / b : 0);
            break;
        }
        case OP_MOD: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, (b != 0) ? a % b : 0);
            break;
        }
        case OP_NEG:
            if (vm->sp > 0) vm->stack[vm->sp - 1] = -vm->stack[vm->sp - 1];
            break;
        case OP_AND: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a & b);
            break;
        }
        case OP_OR: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a | b);
            break;
        }
        case OP_XOR: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a ^ b);
            break;
        }
        case OP_NOT:
            if (vm->sp > 0) vm->stack[vm->sp - 1] = ~vm->stack[vm->sp - 1];
            break;
        case OP_SHL: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, a << (b & 31));
            break;
        }
        case OP_SHR: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, (int)((unsigned)a >> (b & 31)));
            break;
        }
        case OP_CMP_EQ: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, (a == b) ? 1 : 0);
            break;
        }
        case OP_CMP_LT: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, (a < b) ? 1 : 0);
            break;
        }
        case OP_CMP_GT: {
            int b = vm_pop(vm), a = vm_pop(vm);
            vm_push(vm, (a > b) ? 1 : 0);
            break;
        }
        case OP_JMP:
            if (ip < codelen) {
                int offset = (int)(signed char)code[ip];
                ip += offset;
                if (ip < 0) ip = 0;
                if (ip > codelen) ip = codelen;
            }
            break;
        case OP_JZ:
            if (ip < codelen) {
                int val = vm_pop(vm);
                int offset = (int)(signed char)code[ip++];
                if (val == 0) {
                    ip += offset - 1;
                    if (ip < 0) ip = 0;
                    if (ip > codelen) ip = codelen;
                }
            }
            break;
        case OP_JNZ:
            if (ip < codelen) {
                int val = vm_pop(vm);
                int offset = (int)(signed char)code[ip++];
                if (val != 0) {
                    ip += offset - 1;
                    if (ip < 0) ip = 0;
                    if (ip > codelen) ip = codelen;
                }
            }
            break;
        case OP_LOAD:
            if (ip < codelen) {
                int slot = code[ip++] % 64;
                vm_push(vm, vm->memory[slot]);
            }
            break;
        case OP_STORE:
            if (ip < codelen) {
                int slot = code[ip++] % 64;
                vm->memory[slot] = vm_pop(vm);
            }
            break;
        case OP_CALL:
            if (ip < codelen && vm->csp < 32) {
                int target = (int)code[ip++];
                vm->call_stack[vm->csp++] = ip;
                ip = target % codelen;
            }
            break;
        case OP_RET:
            if (vm->csp > 0) {
                ip = vm->call_stack[--vm->csp];
            } else {
                ip = codelen; /* halt */
            }
            break;
        case OP_INC:
            if (vm->sp > 0) vm->stack[vm->sp - 1]++;
            break;
        case OP_DEC:
            if (vm->sp > 0) vm->stack[vm->sp - 1]--;
            break;
        case OP_OVER:
            if (vm->sp >= 2) vm_push(vm, vm->stack[vm->sp - 2]);
            break;
        case OP_ROT: {
            if (vm->sp >= 3) {
                int c = vm->stack[vm->sp - 1];
                int b = vm->stack[vm->sp - 2];
                int a = vm->stack[vm->sp - 3];
                vm->stack[vm->sp - 3] = b;
                vm->stack[vm->sp - 2] = c;
                vm->stack[vm->sp - 1] = a;
            }
            break;
        }
        case OP_NOP:
            break;
        case OP_HALT:
            ip = codelen;
            break;
        default:
            break;
        }
    }

    return vm_peek(vm);
}

/* Generate a pseudo-random but somewhat structured program */
static int generate_program(unsigned char *code, int maxlen) {
    int pos = 0;
    while (pos < maxlen - 2) {
        unsigned int r = bench_lcg_rand();
        unsigned char op = r % NUM_OPS;

        /* Bias toward computational ops for richer execution */
        if (r % 5 == 0) op = OP_PUSH;
        else if (r % 7 == 0) op = OP_ADD;
        else if (r % 11 == 0) op = OP_MUL;
        else if (r % 13 == 0) op = OP_DUP;

        code[pos++] = op;

        /* Ops with operands */
        if (op == OP_PUSH || op == OP_LOAD || op == OP_STORE ||
            op == OP_JMP || op == OP_JZ || op == OP_JNZ || op == OP_CALL) {
            code[pos++] = (unsigned char)(bench_lcg_rand() % 64);
        }
    }
    code[pos++] = OP_HALT;
    return pos;
}

/* Second interpreter: register-based (different dispatch pattern) */
typedef struct {
    int regs[8];
    int flag; /* comparison flag */
} RegVM;

static int regvm_execute(RegVM *rvm, const unsigned char *code, int codelen) {
    int ip = 0;
    int steps = 0;
    int max_steps = codelen * 4;

    while (ip < codelen && steps < max_steps) {
        unsigned char op = code[ip++];
        steps++;
        int ra = (ip < codelen) ? code[ip] % 8 : 0;
        int rb = (ip + 1 < codelen) ? code[ip + 1] % 8 : 0;

        switch (op % 16) {
        case 0: /* LOAD_IMM ra, imm */
            if (ip + 1 < codelen) {
                rvm->regs[ra] = (int)(signed char)code[ip + 1];
                ip += 2;
            }
            break;
        case 1: /* ADD ra, rb */
            rvm->regs[ra] += rvm->regs[rb];
            ip += 2;
            break;
        case 2: /* SUB ra, rb */
            rvm->regs[ra] -= rvm->regs[rb];
            ip += 2;
            break;
        case 3: /* MUL ra, rb */
            rvm->regs[ra] *= rvm->regs[rb];
            ip += 2;
            break;
        case 4: /* AND ra, rb */
            rvm->regs[ra] &= rvm->regs[rb];
            ip += 2;
            break;
        case 5: /* OR ra, rb */
            rvm->regs[ra] |= rvm->regs[rb];
            ip += 2;
            break;
        case 6: /* XOR ra, rb */
            rvm->regs[ra] ^= rvm->regs[rb];
            ip += 2;
            break;
        case 7: /* CMP ra, rb — set flag */
            rvm->flag = rvm->regs[ra] - rvm->regs[rb];
            ip += 2;
            break;
        case 8: /* JZ offset */
            if (rvm->flag == 0 && ip < codelen) {
                ip += (int)(signed char)code[ip];
                if (ip < 0) ip = 0;
                if (ip > codelen) ip = codelen;
            } else {
                ip++;
            }
            break;
        case 9: /* JNZ offset */
            if (rvm->flag != 0 && ip < codelen) {
                ip += (int)(signed char)code[ip];
                if (ip < 0) ip = 0;
                if (ip > codelen) ip = codelen;
            } else {
                ip++;
            }
            break;
        case 10: /* MOV ra, rb */
            rvm->regs[ra] = rvm->regs[rb];
            ip += 2;
            break;
        case 11: /* INC ra */
            rvm->regs[ra]++;
            ip++;
            break;
        case 12: /* DEC ra */
            rvm->regs[ra]--;
            ip++;
            break;
        case 13: /* SHL ra, rb */
            rvm->regs[ra] <<= (rvm->regs[rb] & 31);
            ip += 2;
            break;
        case 14: /* NEG ra */
            rvm->regs[ra] = -rvm->regs[ra];
            ip++;
            break;
        case 15: /* HALT */
            ip = codelen;
            break;
        }
    }

    return rvm->regs[0];
}

static long long workload(unsigned char *code_buf) {
    long long total = 0;

    VM vm;
    RegVM rvm;

    /* Run multiple generated programs through both interpreters */
    for (int p = 0; p < NUM_PROGRAMS; p++) {
        bench_lcg_seed(12345 + p * 1000);
        int codelen = generate_program(code_buf, CODE_SIZE);

        /* Stack-based interpreter */
        vm_init(&vm);
        total += vm_execute(&vm, code_buf, codelen);

        /* Also sum the stack contents */
        for (int i = 0; i < vm.sp; i++) total += vm.stack[i];
        for (int i = 0; i < 64; i++) total += vm.memory[i];

        /* Register-based interpreter on same code */
        memset(&rvm, 0, sizeof(rvm));
        total += regvm_execute(&rvm, code_buf, codelen);
        for (int i = 0; i < 8; i++) total += rvm.regs[i];
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    unsigned char *code_buf = (unsigned char *)malloc(CODE_SIZE);

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(code_buf); });

    free(code_buf);
    return 0;
}
