#define _GNU_SOURCE
#include "suggestions.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#define ALPHA_SIZE 26
#define MIN3(a,b,c) ((a)<(b)?((a)<(c)?(a):(c)):((b)<(c)?(b):(c)))

// --- Trie Structures & Functions ---

typedef struct TrieNode {
    struct TrieNode *children[ALPHA_SIZE];
    uint64_t         frequency;
    int              is_end;
} TrieNode;

static TrieNode *trie_node_create(void) {
    return calloc(1, sizeof(TrieNode));
}

static void trie_insert(TrieNode *root, const char *word, uint64_t freq) {
    TrieNode *curr = root;
    for (int i = 0; word[i]; i++) {
        int idx = tolower((unsigned char)word[i]) - 'a';
        if (idx < 0 || idx >= ALPHA_SIZE) continue;
        if (!curr->children[idx]) {
            curr->children[idx] = trie_node_create();
        }
        curr = curr->children[idx];
    }
    curr->is_end = 1;
    curr->frequency = freq;
}

static void trie_collect(TrieNode *node, char *buf, int depth, Suggestion *out, int *count, int max) {
    if (!node) return;
    if (node->is_end) {
        if (*count < max * 10) { // Collect more then sort/trim
            strncpy(out[*count].word, buf, MAX_WORD_LEN - 1);
            out[*count].word[MAX_WORD_LEN - 1] = '\0';
            out[*count].score = node->frequency;
            out[*count].is_correction = 0;
            (*count)++;
        }
    }
    
    for (int i = 0; i < ALPHA_SIZE; i++) {
        if (node->children[i]) {
            buf[depth] = (char)('a' + i);
            buf[depth + 1] = '\0';
            trie_collect(node->children[i], buf, depth + 1, out, count, max);
        }
    }
}

static void sort_suggestions(Suggestion out[], int count) {
    for (int i = 1; i < count; i++) {
        Suggestion key = out[i];
        int j = i - 1;
        while (j >= 0 && out[j].score < key.score) {
            out[j + 1] = out[j];
            j--;
        }
        out[j + 1] = key;
    }
}

static int trie_prefix_search(TrieNode *root, const char *prefix, Suggestion out[], int max) {
    TrieNode *curr = root;
    for (int i = 0; prefix[i]; i++) {
        int idx = tolower((unsigned char)prefix[i]) - 'a';
        if (idx < 0 || idx >= ALPHA_SIZE) return 0;
        if (!curr->children[idx]) return 0;
        curr = curr->children[idx];
    }
    
    Suggestion pool[MAX_SUGGESTIONS * 20]; // Larger pool for better candidates
    int count = 0;
    char buf[MAX_WORD_LEN];
    strncpy(buf, prefix, MAX_WORD_LEN - 1);
    buf[MAX_WORD_LEN - 1] = '\0';
    
    trie_collect(curr, buf, (int)strlen(prefix), pool, &count, MAX_SUGGESTIONS);
    
    sort_suggestions(pool, count);
    
    int result_count = count < max ? count : max;
    for (int i = 0; i < result_count; i++) {
        out[i] = pool[i];
    }
    return result_count;
}

static void trie_free(TrieNode *node) {
    if (!node) return;
    for (int i = 0; i < ALPHA_SIZE; i++) {
        trie_free(node->children[i]);
    }
    free(node);
}

// --- Levenshtein Distance ---

static int levenshtein(const char *a, const char *b) {
    int la = (int)strlen(a), lb = (int)strlen(b);
    int *prev = calloc((size_t)lb + 1, sizeof(int));
    int *curr = calloc((size_t)lb + 1, sizeof(int));
    for (int j = 0; j <= lb; j++) prev[j] = j;
    for (int i = 1; i <= la; i++) {
        curr[0] = i;
        for (int j = 1; j <= lb; j++) {
            int cost = (tolower((unsigned char)a[i-1]) == tolower((unsigned char)b[j-1])) ? 0 : 1;
            curr[j] = MIN3(prev[j] + 1, curr[j-1] + 1, prev[j-1] + cost);
        }
        int *tmp = prev; prev = curr; curr = tmp;
    }
    int result = prev[lb];
    free(prev); free(curr);
    return result;
}

// --- BK-Tree Structures & Functions ---

typedef struct BKNode {
    char           word[MAX_WORD_LEN];
    uint64_t       frequency;
    struct BKNode *children[32];
    int            child_dist[32];
    int            child_count;
} BKNode;

static BKNode *bknode_create(const char *word, uint64_t freq) {
    BKNode *node = calloc(1, sizeof(BKNode));
    strncpy(node->word, word, MAX_WORD_LEN - 1);
    node->frequency = freq;
    return node;
}

static void bktree_insert(BKNode *root, const char *word, uint64_t freq) {
    int d = levenshtein(root->word, word);
    if (d == 0) return;
    
    for (int i = 0; i < root->child_count; i++) {
        if (root->child_dist[i] == d) {
            bktree_insert(root->children[i], word, freq);
            return;
        }
    }
    
    if (root->child_count < 32) {
        root->children[root->child_count] = bknode_create(word, freq);
        root->child_dist[root->child_count] = d;
        root->child_count++;
    }
}

static void bktree_search(BKNode *node, const char *word, int max_dist, Suggestion *pool, int *count, int max_pool) {
    int d = levenshtein(node->word, word);
    if (d > 0 && d <= max_dist) {
        if (*count < max_pool) {
            strncpy(pool[*count].word, node->word, MAX_WORD_LEN - 1);
            pool[*count].word[MAX_WORD_LEN - 1] = '\0';
            pool[*count].score = node->frequency;
            pool[*count].is_correction = 1;
            (*count)++;
        }
    }
    
    for (int i = 0; i < node->child_count; i++) {
        if (node->child_dist[i] >= d - max_dist && node->child_dist[i] <= d + max_dist) {
            bktree_search(node->children[i], word, max_dist, pool, count, max_pool);
        }
    }
}

static void bktree_free(BKNode *node) {
    if (!node) return;
    for (int i = 0; i < node->child_count; i++) {
        bktree_free(node->children[i]);
    }
    free(node);
}

// --- SuggestionEngine Implementation ---

struct SuggestionEngine {
    TrieNode *trie_root;
    BKNode   *bk_root;
    int       word_count;
};

SuggestionEngine *suggestions_engine_create(const char *words_freq_path) {
    FILE *f = fopen(words_freq_path, "r");
    if (!f) {
        fprintf(stderr, "typd: words.freq not found: %s\n", words_freq_path);
        return NULL;
    }
    
    SuggestionEngine *engine = calloc(1, sizeof(SuggestionEngine));
    engine->trie_root = trie_node_create();
    
    char word[MAX_WORD_LEN];
    uint64_t freq;
    while (fscanf(f, "%63s\t%lu", word, &freq) == 2) {
        for (int i = 0; word[i]; i++) word[i] = (char)tolower((unsigned char)word[i]);
        trie_insert(engine->trie_root, word, freq);
        if (!engine->bk_root) {
            engine->bk_root = bknode_create(word, freq);
        } else {
            bktree_insert(engine->bk_root, word, freq);
        }
        engine->word_count++;
    }
    
    fclose(f);
    fprintf(stderr, "typd: loaded %d words\n", engine->word_count);
    return engine;
}

void suggestions_engine_destroy(SuggestionEngine *engine) {
    if (!engine) return;
    trie_free(engine->trie_root);
    bktree_free(engine->bk_root);
    free(engine);
}

int suggestions_prefix(SuggestionEngine *engine, const char *prefix, Suggestion out[MAX_SUGGESTIONS]) {
    if (!engine || !prefix || !prefix[0]) return 0;
    return trie_prefix_search(engine->trie_root, prefix, out, MAX_SUGGESTIONS);
}

int suggestions_correct(SuggestionEngine *engine, const char *word, Suggestion out[MAX_SUGGESTIONS]) {
    if (!engine || !word || !word[0]) return 0;
    
    // Check if word in Trie
    TrieNode *curr = engine->trie_root;
    int found = 1;
    for (int i = 0; word[i]; i++) {
        int idx = tolower((unsigned char)word[i]) - 'a';
        if (idx < 0 || idx >= ALPHA_SIZE || !curr->children[idx]) {
            found = 0;
            break;
        }
        curr = curr->children[idx];
    }
    if (found && curr->is_end) return 0;
    
    Suggestion pool[100];
    int count = 0;
    bktree_search(engine->bk_root, word, 2, pool, &count, 100);
    
    sort_suggestions(pool, count);
    
    int result_count = count < MAX_SUGGESTIONS ? count : MAX_SUGGESTIONS;
    for (int i = 0; i < result_count; i++) {
        out[i] = pool[i];
    }
    return result_count;
}
