#include <assert.h>
#include <stdio.h>

#include <rune.h>

/** 
 * A custom C function that interacts with Rune. This is registered below with
 * rune_module_function.
 */
void custom_function(rune_stack *stack, uintptr_t count, rune_vm_error *e) {
    rune_value value = rune_value_unit();

    if (count != 1) {
        rune_vm_error_bad_argument_count(e, count, 1);
        return;
    }

    // Note: Error will be automatically propagated since it's used as an output
    // argument.
    if (!rune_stack_pop_value(stack, &value, e)) {
        return;
    }

    int64_t integer = 0;

    if (!rune_value_as_integer(&value, &integer)) {
        rune_vm_error_bad_argument_at(e, 0, &value, RUNE_INTEGER_TYPE);
        return;
    }

    rune_stack_push_unit(stack);
    rune_stack_push_integer(stack, integer * 10);
    rune_stack_push_tuple(stack, 2, e);
}

int main() {
    rune_context context = rune_context_new();
    rune_module module = rune_module_new();
    rune_runtime_context runtime = rune_runtime_context_new();
    rune_sources sources = rune_sources_new();
    rune_standard_stream out = rune_standard_stream_stderr(RUNE_COLOR_CHOICE_ALWAYS);
    rune_unit unit = rune_unit_new();
    rune_vm vm = rune_vm_new();
    rune_vm_error error = rune_vm_error_new();
    rune_context_error context_error = rune_context_error_new();

    if (!rune_module_function(&module, "test", custom_function, &context_error)) {
        rune_context_error_emit(&context_error, &out);
        goto EXIT;
    }

    if (!rune_context_install(&context, &module, &context_error)) {
        rune_context_error_emit(&context_error, &out);
        goto EXIT;
    }

    rune_module_free(&module);

    rune_source source = rune_source_new("<in>", "pub fn main(n) { test(n) }");
    assert(rune_sources_insert(&sources, &source));
    rune_source_free(&source);

    rune_diagnostics diag = rune_diagnostics_new();

    rune_build build = rune_build_prepare(&sources);
    rune_build_with_diagnostics(&build, &diag);
    rune_build_with_context(&build, &context);

    bool ok = rune_build_build(&build, &unit);

    if (!rune_diagnostics_is_empty(&diag)) {
        assert(rune_diagnostics_emit(&diag, &out, &sources));
    }

    rune_diagnostics_free(&diag);

    if (!ok) {
        goto EXIT;
    }

    assert(rune_context_runtime(&context, &runtime));
    assert(rune_vm_setup(&vm, &runtime, &unit));

    rune_hash entry = rune_hash_name("main");

    if (!rune_vm_set_entrypoint(&vm, entry, 1, &error)) {
        assert(rune_vm_error_emit(&error, &out, &sources));
        goto EXIT;
    }

    rune_stack_push_integer(rune_vm_stack_mut(&vm), 42);
    rune_value ret = rune_value_unit();

    if (!rune_vm_complete(&vm, &ret, &error)) {
        assert(rune_vm_error_emit(&error, &out, &sources));
    }

    int64_t output = 0;

    if (rune_value_as_integer(&ret, &output)) {
        printf("output = %lld\n", output);
    } else {
        rune_hash type_hash = rune_hash_empty();

        if (rune_value_type_hash(&ret, &type_hash, &error)) {
            printf("output = %lld\n", type_hash);
        } else {
            printf("output = ?\n");
        }
    }

    rune_value_free(&ret);

EXIT:
    rune_context_free(&context);
    rune_module_free(&module);
    rune_runtime_context_free(&runtime);
    rune_sources_free(&sources);
    rune_standard_stream_free(&out);
    rune_unit_free(&unit);
    rune_vm_error_free(&error);
    rune_vm_free(&vm);
    return 0;
}
