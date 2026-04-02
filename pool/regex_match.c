/*
 * Targets: jump-threading (state transitions), simplifycfg (branch chains),
 * early-cse (repeated state checks), inline (matcher helpers),
 * loop-unroll (character scanning loops).
 *
 * Simple NFA-based regex matcher with character classes and quantifiers.
 */
#include "bench_timing.h"

#define TEXT_SIZE 2000
#define MAX_STATES 64
#define MAX_TRANSITIONS 256

/* ---- NFA structures ---- */

typedef enum {
    MATCH_LITERAL,    /* match exact character */
    MATCH_ANY,        /* match any character (.) */
    MATCH_DIGIT,      /* match [0-9] */
    MATCH_ALPHA,      /* match [a-zA-Z] */
    MATCH_ALNUM,      /* match [a-zA-Z0-9] */
    MATCH_SPACE,      /* match whitespace */
    MATCH_WORD,       /* match [a-zA-Z0-9_] */
    MATCH_RANGE,      /* match [lo-hi] */
    MATCH_EPSILON     /* epsilon transition (no input consumed) */
} MatchType;

typedef struct {
    MatchType type;
    char literal;
    char range_lo, range_hi;
    int next_state;
    int alt_state;     /* for epsilon splits (alternation) */
} Transition;

typedef struct {
    Transition transitions[MAX_TRANSITIONS];
    int ntrans;
    int start_state;
    int accept_state;
} NFA;

/* ---- Character class helpers ---- */

static int is_digit(char c) { return c >= '0' && c <= '9'; }
static int is_alpha(char c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); }
static int is_alnum(char c) { return is_digit(c) || is_alpha(c); }
static int is_space(char c) { return c == ' ' || c == '\t' || c == '\n' || c == '\r'; }
static int is_word(char c) { return is_alnum(c) || c == '_'; }

static int match_char(const Transition *t, char c) {
    switch (t->type) {
    case MATCH_LITERAL: return c == t->literal;
    case MATCH_ANY:     return c != '\0';
    case MATCH_DIGIT:   return is_digit(c);
    case MATCH_ALPHA:   return is_alpha(c);
    case MATCH_ALNUM:   return is_alnum(c);
    case MATCH_SPACE:   return is_space(c);
    case MATCH_WORD:    return is_word(c);
    case MATCH_RANGE:   return c >= t->range_lo && c <= t->range_hi;
    case MATCH_EPSILON: return 0; /* epsilon doesn't consume input */
    }
    return 0;
}

/* ---- NFA simulation ---- */

typedef struct {
    int states[MAX_STATES];
    int count;
} StateSet;

static void stateset_init(StateSet *ss) { ss->count = 0; }

static int stateset_contains(const StateSet *ss, int state) {
    for (int i = 0; i < ss->count; i++) {
        if (ss->states[i] == state) return 1;
    }
    return 0;
}

static void stateset_add(StateSet *ss, int state) {
    if (ss->count < MAX_STATES && !stateset_contains(ss, state)) {
        ss->states[ss->count++] = state;
    }
}

/* Follow epsilon transitions (epsilon closure) */
static void epsilon_closure(const NFA *nfa, StateSet *ss) {
    int changed = 1;
    while (changed) {
        changed = 0;
        for (int i = 0; i < ss->count; i++) {
            int s = ss->states[i];
            /* Check all transitions from state s */
            for (int t = 0; t < nfa->ntrans; t++) {
                if (nfa->transitions[t].type == MATCH_EPSILON &&
                    nfa->transitions[t].next_state == s) {
                    /* This is an epsilon transition TO state s,
                       but we want FROM state s */
                }
            }
            /* Simplified: check if state s has an alt_state (epsilon branch) */
            for (int t = 0; t < nfa->ntrans; t++) {
                if (nfa->transitions[t].type == MATCH_EPSILON) {
                    int from = t; /* use transition index as implicit from-state */
                    if (from < nfa->ntrans &&
                        stateset_contains(ss, nfa->transitions[t].next_state)) {
                        if (nfa->transitions[t].alt_state >= 0 &&
                            !stateset_contains(ss, nfa->transitions[t].alt_state)) {
                            stateset_add(ss, nfa->transitions[t].alt_state);
                            changed = 1;
                        }
                    }
                }
            }
        }
    }
}

/* Simulate NFA on input text. Returns 1 if match found. */
static int nfa_match(const NFA *nfa, const char *text, int textlen) {
    StateSet current, next;
    stateset_init(&current);
    stateset_add(&current, nfa->start_state);
    epsilon_closure(nfa, &current);

    for (int pos = 0; pos < textlen; pos++) {
        stateset_init(&next);

        for (int i = 0; i < current.count; i++) {
            int s = current.states[i];
            /* Find transitions from state s */
            for (int t = 0; t < nfa->ntrans; t++) {
                if (nfa->transitions[t].next_state == s &&
                    nfa->transitions[t].type != MATCH_EPSILON) {
                    if (match_char(&nfa->transitions[t], text[pos])) {
                        stateset_add(&next, nfa->transitions[t].alt_state >= 0
                                     ? nfa->transitions[t].alt_state : s + 1);
                    }
                }
            }
        }

        epsilon_closure(nfa, &next);

        if (next.count == 0) return 0; /* dead end */

        /* Copy next -> current */
        current = next;
    }

    /* Check if any current state is the accept state */
    return stateset_contains(&current, nfa->accept_state);
}

/* ---- Simple pattern matchers (no NFA, direct code) ---- */

/* Match a literal string anywhere in text */
static int find_literal(const char *text, int textlen,
                        const char *pattern, int patlen) {
    int count = 0;
    for (int i = 0; i <= textlen - patlen; i++) {
        int match = 1;
        for (int j = 0; j < patlen; j++) {
            if (text[i + j] != pattern[j]) {
                match = 0;
                break;
            }
        }
        if (match) count++;
    }
    return count;
}

/* Match digit runs: find all runs of consecutive digits */
static int find_digit_runs(const char *text, int textlen) {
    int count = 0;
    int in_run = 0;
    for (int i = 0; i < textlen; i++) {
        if (is_digit(text[i])) {
            if (!in_run) { count++; in_run = 1; }
        } else {
            in_run = 0;
        }
    }
    return count;
}

/* Match word boundaries: count transitions from non-word to word char */
static int find_word_boundaries(const char *text, int textlen) {
    int count = 0;
    for (int i = 0; i < textlen; i++) {
        int prev_word = (i > 0) ? is_word(text[i - 1]) : 0;
        int curr_word = is_word(text[i]);
        if (!prev_word && curr_word) count++;
    }
    return count;
}

/* Simple glob match: * matches any sequence, ? matches any single char */
static int glob_match(const char *pattern, const char *text) {
    const char *p = pattern, *t = text;
    const char *star_p = NULL, *star_t = NULL;

    while (*t) {
        if (*p == *t || *p == '?') {
            p++; t++;
        } else if (*p == '*') {
            star_p = p++;
            star_t = t;
        } else if (star_p) {
            p = star_p + 1;
            t = ++star_t;
        } else {
            return 0;
        }
    }
    while (*p == '*') p++;
    return *p == '\0';
}

/* Count matches of character class pattern [class]+ */
static int count_class_runs(const char *text, int textlen,
                            int (*class_fn)(char)) {
    int count = 0;
    int total_len = 0;
    int in_run = 0;
    for (int i = 0; i < textlen; i++) {
        if (class_fn(text[i])) {
            if (!in_run) { count++; in_run = 1; }
            total_len++;
        } else {
            in_run = 0;
        }
    }
    return count + total_len;
}

/* Simple regex-like: match pattern with . and * (dot-star) */
static int dotstar_match(const char *pattern, int patlen,
                         const char *text, int textlen) {
    /* DP match: dp[i][j] = can pattern[0..i) match text[0..j)? */
    /* Use two rows to save memory */
    int prev[256], curr[256];
    if (patlen > 255 || textlen > 255) return 0;

    memset(prev, 0, (textlen + 1) * sizeof(int));
    prev[0] = 1;

    /* Handle leading * patterns */
    for (int i = 0; i < patlen; i++) {
        if (pattern[i] == '*' && i > 0) prev[0] = prev[0]; /* * can match empty */
        else break;
    }

    for (int i = 1; i <= patlen; i++) {
        memset(curr, 0, (textlen + 1) * sizeof(int));
        char pc = pattern[i - 1];

        if (pc == '*' && i >= 2) {
            /* * means zero or more of preceding element */
            curr[0] = prev[0]; /* zero repetitions */
            char prev_pc = pattern[i - 2];
            for (int j = 1; j <= textlen; j++) {
                curr[j] = prev[j]; /* zero repetitions of prev_pc */
                if (curr[j - 1] &&
                    (prev_pc == '.' || text[j - 1] == prev_pc)) {
                    curr[j] = 1; /* one more repetition */
                }
            }
        } else {
            for (int j = 1; j <= textlen; j++) {
                if (pc == '.' || pc == text[j - 1]) {
                    curr[j] = prev[j - 1];
                }
            }
        }

        /* Swap */
        memcpy(prev, curr, (textlen + 1) * sizeof(int));
    }

    return prev[textlen];
}

/* Build a simple NFA from a basic pattern */
static void build_simple_nfa(NFA *nfa, const char *pattern, int patlen) {
    nfa->ntrans = 0;
    nfa->start_state = 0;
    nfa->accept_state = patlen;

    for (int i = 0; i < patlen && nfa->ntrans < MAX_TRANSITIONS; i++) {
        Transition *t = &nfa->transitions[nfa->ntrans++];
        t->next_state = i;
        t->alt_state = i + 1;

        switch (pattern[i]) {
        case '.': t->type = MATCH_ANY; break;
        case 'd': t->type = MATCH_DIGIT; break;
        case 'w': t->type = MATCH_WORD; break;
        case 's': t->type = MATCH_SPACE; break;
        default:
            t->type = MATCH_LITERAL;
            t->literal = pattern[i];
            break;
        }
    }
}

static long long workload(char *text) {
    long long total = 0;

    /* Direct pattern matching */
    total += find_literal(text, TEXT_SIZE, "the", 3);
    total += find_literal(text, TEXT_SIZE, "and", 3);
    total += find_literal(text, TEXT_SIZE, "is", 2);
    total += find_literal(text, TEXT_SIZE, "of", 2);
    total += find_literal(text, TEXT_SIZE, "abcde", 5);

    /* Character class matchers */
    total += find_digit_runs(text, TEXT_SIZE);
    total += find_word_boundaries(text, TEXT_SIZE);
    total += count_class_runs(text, TEXT_SIZE, is_digit);
    total += count_class_runs(text, TEXT_SIZE, is_alpha);
    total += count_class_runs(text, TEXT_SIZE, is_space);
    total += count_class_runs(text, TEXT_SIZE, is_word);

    /* Glob matching on substrings */
    char sub[64];
    for (int i = 0; i < 20; i++) {
        int start = (bench_lcg_rand() % (TEXT_SIZE - 60));
        memcpy(sub, text + start, 60);
        sub[60] = '\0';
        total += glob_match("*the*", sub);
        total += glob_match("?a*d?", sub);
        total += glob_match("*[0-9]*", sub);
    }

    /* Dot-star regex matching */
    for (int i = 0; i < 10; i++) {
        int start = (bench_lcg_rand() % (TEXT_SIZE - 30));
        total += dotstar_match("t.e", 3, text + start, 30);
        total += dotstar_match("a.*b", 4, text + start, 30);
        total += dotstar_match(".o.", 3, text + start, 30);
    }

    /* NFA matching */
    NFA nfa;
    build_simple_nfa(&nfa, "the", 3);
    for (int i = 0; i < TEXT_SIZE - 10; i += 10) {
        total += nfa_match(&nfa, text + i, 10);
    }

    build_simple_nfa(&nfa, "d.d", 3);  /* digit-any-digit */
    for (int i = 0; i < TEXT_SIZE - 10; i += 10) {
        total += nfa_match(&nfa, text + i, 10);
    }

    build_simple_nfa(&nfa, "www", 3);  /* three word chars */
    for (int i = 0; i < TEXT_SIZE - 10; i += 10) {
        total += nfa_match(&nfa, text + i, 10);
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    char *text = (char *)malloc(TEXT_SIZE + 1);

    /* Generate English-like text */
    bench_lcg_seed(12345);
    const char *words[] = {
        "the", "and", "is", "of", "to", "in", "a", "that",
        "it", "for", "was", "on", "are", "with", "as", "at",
        "be", "this", "have", "from", "or", "one", "had", "by",
        "not", "but", "what", "all", "were", "when", "123", "42"
    };
    int nwords = 32;
    int pos = 0;
    while (pos < TEXT_SIZE - 20) {
        int widx = bench_lcg_rand() % nwords;
        const char *w = words[widx];
        while (*w && pos < TEXT_SIZE) {
            text[pos++] = *w++;
        }
        if (pos < TEXT_SIZE) text[pos++] = ' ';
    }
    text[TEXT_SIZE] = '\0';

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(text); });

    free(text);
    return 0;
}
