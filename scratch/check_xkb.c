#include <xkbcommon/xkbcommon.h>
#include <stdio.h>
#include <stdlib.h>

int main() {
    struct xkb_context *ctx = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
    struct xkb_rule_names names = { .layout = "us" };
    struct xkb_keymap *keymap = xkb_keymap_new_from_names(ctx, &names, XKB_KEYMAP_COMPILE_NO_FLAGS);
    char *str = xkb_keymap_get_as_string(keymap, XKB_KEYMAP_FORMAT_TEXT_V1);
    printf("%s\n", str);
    free(str);
    xkb_keymap_unref(keymap);
    xkb_context_unref(ctx);
    return 0;
}
