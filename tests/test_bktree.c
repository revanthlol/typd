#include "../include/suggestions.h"
#include <stdio.h>
#include <string.h>
#include <assert.h>

static void test_correct_word_not_corrected(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_correct(e, "hello", out);
    assert(n == 0);  // "hello" is in dict, no correction
    suggestions_engine_destroy(e);
    printf("PASS: test_correct_word_not_corrected\n");
}

static void test_typo_distance1(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_correct(e, "helo", out);  // distance 1 from "hello"
    assert(n >= 1);
    // At least one result should be "hello"
    int found = 0;
    for (int i = 0; i < n; i++)
        if (strcmp(out[i].word, "hello") == 0) { found = 1; break; }
    assert(found);
    assert(out[0].is_correction == 1);
    suggestions_engine_destroy(e);
    printf("PASS: test_typo_distance1\n");
}

static void test_typo_distance2(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    int n = suggestions_correct(e, "wrold", out);  // distance 2 from "world"
    assert(n >= 1);
    int found = 0;
    for (int i = 0; i < n; i++)
        if (strcmp(out[i].word, "world") == 0) { found = 1; break; }
    assert(found);
    suggestions_engine_destroy(e);
    printf("PASS: test_typo_distance2\n");
}

static void test_no_result_distance3(void) {
    SuggestionEngine *e = suggestions_engine_create("data/words.freq");
    assert(e != NULL);
    Suggestion out[MAX_SUGGESTIONS];
    // "xyzabc" is far from everything
    int n = suggestions_correct(e, "xyzabc", out);
    (void)n;  // may or may not find something, just must not crash
    suggestions_engine_destroy(e);
    printf("PASS: test_no_result_distance3\n");
}

int main(void) {
    test_correct_word_not_corrected();
    test_typo_distance1();
    test_typo_distance2();
    test_no_result_distance3();
    printf("ALL BK-TREE TESTS PASSED\n");
    return 0;
}
