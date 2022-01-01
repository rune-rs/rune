#include <assert.h>
#include <stdio.h>

#include <rune.h>

int main() {
    rune_value a = rune_value_integer(42);
    rune_value b = rune_value_bool(false);
    rune_vm_error error = rune_vm_error_new();

    assert(rune_value_type_hash_or_empty(&a) == RUNE_INTEGER_TYPE_HASH);
    assert(rune_value_type_hash_or_empty(&b) == RUNE_BOOL_TYPE_HASH);

    rune_vm_error_free(&error);
    return 0;
}
