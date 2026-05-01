#include "../include/suggestions.h"
#include <stdio.h>
#include <string.h>
#include <assert.h>

static void test_prefix_hel(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_prefix(e, "hel", out);
    assert(n >= 1);
    // First result must be "hello" (highest frequency among hel*)
    assert(strcmp(out[0].word, "hello") == 0);
    assert(out[0].is_correction == 0);
    suggestions_engine_destroy(e);
    printf("PASS: test_prefix_hel\n");
}

static void test_prefix_empty(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_prefix(e, "", out);
    assert(n == 0);
    suggestions_engine_destroy(e);
    printf("PASS: test_prefix_empty\n");
}

static void test_prefix_no_match(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_prefix(e, "zzzzz", out);
    assert(n == 0);
    suggestions_engine_destroy(e);
    printf("PASS: test_prefix_no_match\n");
}

static void test_prefix_returns_max3(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_prefix(e, "a", out);  // many words start with 'a'
    assert(n <= MAX_SUGGESTIONS);
    suggestions_engine_destroy(e);
    printf("PASS: test_prefix_returns_max3\n");
}

int main(void) {
    test_prefix_hel();
    test_prefix_empty();
    test_prefix_no_match();
    test_prefix_returns_max3();
    printf("ALL TRIE TESTS PASSED\n");
    return 0;
}
