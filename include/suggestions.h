#pragma once
#include <stdint.h>

#define MAX_WORD_LEN     64
#define MAX_SUGGESTIONS   3

// Single suggestion result
typedef struct {
    char     word[MAX_WORD_LEN];
    uint64_t score;        // frequency score (higher = better)
    int      is_correction; // 1 if from BK-Tree fuzzy match, 0 if prefix match
} Suggestion;

// Opaque engine handle
typedef struct SuggestionEngine SuggestionEngine;

// Lifecycle
SuggestionEngine *suggestions_engine_create(const char *words_freq_path);
void              suggestions_engine_destroy(SuggestionEngine *engine);

// Query — prefix autocomplete (called while typing)
// Returns number of results written into out[] (0..MAX_SUGGESTIONS)
int suggestions_prefix(SuggestionEngine *engine,
                       const char *prefix,
                       Suggestion out[MAX_SUGGESTIONS]);

// Query — fuzzy correction (called when spacebar pressed, word complete)
// Returns 0 if word is in dictionary (no correction needed)
// Returns N>0 if corrections found, written into out[]
int suggestions_correct(SuggestionEngine *engine,
                        const char *word,
                        Suggestion out[MAX_SUGGESTIONS]);
