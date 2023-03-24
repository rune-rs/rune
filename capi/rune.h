#ifndef RUNE_H
#define RUNE_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

/**
 * The color choice.
 */
enum rune_color_choice
#ifdef __cplusplus
  : uintptr_t
#endif // __cplusplus
 {
  /**
   * Try very hard to emit colors. This includes emitting ANSI colors on
   * Windows if the console API is unavailable.
   */
  RUNE_COLOR_CHOICE_ALWAYS = 1,
  /**
   * AlwaysAnsi is like Always, except it never tries to use anything other
   * than emitting ANSI color codes.
   */
  RUNE_COLOR_CHOICE_ALWAYS_ANSI = 2,
  /**
   * Try to use colors, but don't force the issue. If the console isn't
   * available on Windows, or if TERM=dumb, or if `NO_COLOR` is defined, for
   * example, then don't use colors.
   */
  RUNE_COLOR_CHOICE_AUTO = 3,
  /**
   * Never emit colors.
   */
  RUNE_COLOR_CHOICE_NEVER = 4,
};
#ifndef __cplusplus
typedef uintptr_t rune_color_choice;
#endif // __cplusplus

/**
 * A collection of sources.
 */
typedef struct {
  uint8_t repr[24];
} rune_sources;

/**
 * A context.
 */
typedef struct {
  uint8_t repr[656];
} rune_context;

/**
 * Build diagnostics.
 */
typedef struct {
  uint8_t repr[32];
} rune_diagnostics;

/**
 * Prepare a build.
 */
typedef struct {
  rune_sources *sources;
  rune_context *context;
  rune_diagnostics *diagnostics;
} rune_build;

/**
 * A rune source file.
 */
typedef struct {
  uint8_t repr[8];
} rune_unit;

/**
 * A module with custom functions and the like.
 */
typedef struct {
  uint8_t repr[408];
} rune_module;

/**
 * An error that can be raised by a virtual machine.
 *
 * This must be declared with [rune_context_error_new] and must be freed with
 * [rune_context_error_free].
 *
 * \code{.c}
 * int main() {
 *     rune_context_error error = rune_context_error_new();
 *
 *     // ...
 *
 *     rune_context_error_free(&error);
 * }
 * \endcode
 */
typedef struct {
  uint8_t repr[152];
} rune_context_error;

/**
 * A runtime context.
 */
typedef struct {
  uint8_t repr[8];
} rune_runtime_context;

/**
 * A standard stream.
 */
typedef struct {
  uint8_t repr[88];
} rune_standard_stream;

/**
 * An opaque hash.
 */
typedef uint64_t rune_hash;

/**
 * A rune source file.
 */
typedef struct {
  uint8_t repr[72];
} rune_source;

/**
 * The stack of a virtual machine.
 */
typedef struct {
  uint8_t repr[32];
} rune_stack;

/**
 * A value in a virtual machine.
 */
typedef struct {
  uint8_t repr[16];
} rune_value;

/**
 * An error that can be raised by a virtual machine.
 *
 * This must be declared with [rune_vm_error_new] and must be freed with
 * [rune_vm_error_free].
 *
 * \code{.c}
 * int main() {
 *     rune_vm_error error = rune_vm_error_new();
 *
 *     // ...
 *
 *     rune_vm_error_free(&error);
 * }
 * \endcode
 */
typedef struct {
  uint8_t repr[8];
} rune_vm_error;

/**
 * The signature of a custom function.
 *
 * Where `stack` is the stack being interacted with and `count` are the number
 * of arguments passed in.
 */
typedef void (*Function)(rune_stack *stack, uintptr_t count, rune_vm_error*);

/**
 * A virtual machine.
 */
typedef struct {
  uint8_t repr[80];
} rune_vm;

typedef struct {
  const void *inner;
} rune_static_type;

/**
 * The type hash of an integer.
 */
#define RUNE_INTEGER_TYPE_HASH 13490401188435821026ULL

/**
 * The type hash of a boolean.
 */
#define RUNE_BOOL_TYPE_HASH 13721341357821314905ULL

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

extern const rune_static_type RUNE_BOOL_TYPE;

extern const rune_static_type RUNE_BYTES_TYPE;

extern const rune_static_type RUNE_BYTE_TYPE;

extern const rune_static_type RUNE_CHAR_TYPE;

extern const rune_static_type RUNE_FLOAT_TYPE;

extern const rune_static_type RUNE_FORMAT_TYPE;

extern const rune_static_type RUNE_FUNCTION_TYPE;

extern const rune_static_type RUNE_FUTURE_TYPE;

extern const rune_static_type RUNE_GENERATOR_STATE_TYPE;

extern const rune_static_type RUNE_GENERATOR_TYPE;

extern const rune_static_type RUNE_INTEGER_TYPE;

extern const rune_static_type RUNE_ITERATOR_TYPE;

extern const rune_static_type RUNE_OBJECT_TYPE;

extern const rune_static_type RUNE_OPTION_TYPE;

extern const rune_static_type RUNE_RANGE_TYPE;

extern const rune_static_type RUNE_RESULT_TYPE;

extern const rune_static_type RUNE_STREAM_TYPE;

extern const rune_static_type RUNE_STRING_TYPE;

extern const rune_static_type RUNE_TUPLE_TYPE;

extern const rune_static_type RUNE_TYPE;

extern const rune_static_type RUNE_UNIT_TYPE;

extern const rune_static_type RUNE_VEC_TYPE;

/**
 * Prepare a new build.
 */
rune_build rune_build_prepare(rune_sources *sources);

/**
 * Associate a context with the build.
 *
 * # Safety
 *
 * Must be called with a `build` argument that has been setup with
 * [rune_build_prepare] and a `context` that has been allocated with
 * [rune_context_new][crate::rune_context_new].
 */
void rune_build_with_context(rune_build *build, rune_context *context);

/**
 * Associate diagnostics with the build.
 *
 * # Safety
 *
 * Must be called with a `build` argument that has been setup with
 * [rune_build_prepare] and a `diagnostics` that has been allocated with
 * [rune_diagnostics_new][crate::rune_diagnostics_new].
 */
void rune_build_with_diagnostics(rune_build *build, rune_diagnostics *diagnostics);

/**
 * Perform a build.
 *
 * On a successful returns `true` and sets `unit` to the newly allocated unit.
 * Any old unit present will be de-allocated.
 * Otherwise the `unit` argument is left alone.
 *
 * # Safety
 *
 * Must be called with a `build` argument that has been setup with
 * [rune_build_prepare] and a `unit` that has been allocated with
 * [rune_unit_new][crate::rune_unit_new].
 */
bool rune_build_build(rune_build *build, rune_unit *unit);

/**
 * Construct a new context.
 */
rune_context rune_context_new(void);

/**
 * Free a context. After it's been freed the context is no longer valid.
 *
 * # Safety
 *
 * Must be called with a context allocated through [rune_context_new].
 */
void rune_context_free(rune_context *context);

/**
 * Install the given module into the current context.
 *
 * Returns `false` if either context or `module` is not present or if
 * installation fails.
 *
 * # Safety
 *
 * The current `context` must have been allocated with [rune_context_new].
 */
bool rune_context_install(rune_context *context,
                          const rune_module *module,
                          rune_context_error *error);

/**
 * Construct a runtime context from the current context.
 *
 * # Safety
 *
 * Function must be called with a `context` object allocated by
 * [rune_context_new] and a valid `runtime` argument allocated with
 * [rune_runtime_context_new][crate::rune_runtime_context_new].
 */
bool rune_context_runtime(const rune_context *context, rune_runtime_context *runtime);

/**
 * Construct an empty [ContextError].
 */
rune_context_error rune_context_error_new(void);

/**
 * Free the given context error.
 *
 * # Safety
 *
 * Must be called with an error that has been allocated with
 * [rune_context_error_new].
 */
void rune_context_error_free(rune_context_error *error);

/**
 * Emit diagnostics to the given stream if the error is set. If the error is
 * not set nothing will be emitted.
 *
 * TODO: propagate I/O errors somehow.
 *
 * # Safety
 *
 * Must be called with an error that has been allocated with
 * [rune_context_error_new].
 */
bool rune_context_error_emit(const rune_context_error *error, rune_standard_stream *stream);

/**
 * Construct a new build diagnostics instance.
 *
 * Used with [rn_build_diagnostics][crate:rn_build_diagnostics].
 */
rune_diagnostics rune_diagnostics_new(void);

/**
 * Free a build diagnostics instance.
 *
 * # Safety
 *
 * Function must be called with a diagnostics object allocated by
 * [rune_diagnostics_new].
 */
void rune_diagnostics_free(rune_diagnostics *diagnostics);

/**
 * Test if diagnostics is empty. Will do nothing if the diagnostics object is
 * not present.
 *
 * # Safety
 *
 * Function must be called with a diagnostics object allocated by
 * [rune_diagnostics_new].
 */
bool rune_diagnostics_is_empty(const rune_diagnostics *diagnostics);

/**
 * Emit diagnostics to the given stream.
 *
 * TODO: propagate I/O errors somehow.
 *
 * # Safety
 *
 * Function must be called with a diagnostics object allocated by
 * [rune_diagnostics_new] and a valid `stream` and `sources` argument.
 */
bool rune_diagnostics_emit(const rune_diagnostics *diagnostics,
                           rune_standard_stream *stream,
                           const rune_sources *sources);

/**
 * Construct the empty hash.
 */
rune_hash rune_hash_empty(void);

/**
 * Generate a hash corresponding to the given name.
 *
 * Returns an empty hash that can be tested with [rn_hash_is_empty].
 *
 * # Safety
 *
 * Function must be called with a non-NULL `name` argument.
 */
rune_hash rune_hash_name(const char *name);

/**
 * Test if the hash is empty.
 */
bool rune_hash_is_empty(rune_hash hash);

/**
 * Construct a compile source.
 *
 * Returns an empty source if the name or the source is not valid UTF-8.
 *
 * # Safety
 *
 * Must be called a `name` and `source` argument that points to valid
 * NULL-terminated UTF-8 strings.
 */
rune_source rune_source_new(const char *name, const char *source);

/**
 * Free a compile source. Does nothing if it has already been freed.
 *
 * # Safety
 *
 * Must be called with a `source` that has been allocation with
 * [rune_source_new].
 */
void rune_source_free(rune_source *source);

/**
 * Construct a new [rn_sources] object.
 */
rune_sources rune_sources_new(void);

/**
 * Insert a source to be compiled. Once inserted, they are part of sources
 * collection and do not need to be freed.
 *
 * Returns `true` if the source was successfully inserted. Otherwise it means
 * that the provided source was empty.
 *
 * # Safety
 *
 * Must be called with a `sources` collection allocated with
 * [rune_sources_new].
 */
bool rune_sources_insert(rune_sources *sources, rune_source *source);

/**
 * Free a sources collection. After it's been freed the collection is no longer
 * valid.
 *
 * # Safety
 *
 * Must be called with a `sources` collection allocated with
 * [rune_sources_new].
 */
void rune_sources_free(rune_sources *sources);

/**
 * Construct a standard stream for stdout.
 */
rune_standard_stream rune_standard_stream_stdout(rune_color_choice color_choice);

/**
 * Construct a standard stream for stderr.
 */
rune_standard_stream rune_standard_stream_stderr(rune_color_choice color_choice);

/**
 * Free a standard stream.
 *
 * # Safety
 *
 * This must be called with a `standard_stream` that has been allocated with
 * functions such as [rune_standard_stream_stdout] or
 * [rune_standard_stream_stderr].
 */
void rune_standard_stream_free(rune_standard_stream *standard_stream);

/**
 * Push a value onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push(rune_stack *stack, rune_value value);

/**
 * Push a unit value onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push_unit(rune_stack *stack);

/**
 * Push a value with the given type onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push_bool(rune_stack *stack, bool value);

/**
 * Push a value with the given type onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push_byte(rune_stack *stack, uint8_t value);

/**
 * Push a value with the given type onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push_integer(rune_stack *stack, int64_t value);

/**
 * Push a value with the given type onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push_float(rune_stack *stack, double value);

/**
 * Push a value with the given type onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
void rune_stack_push_type(rune_stack *stack, rune_hash value);

/**
 * Push a character onto the stack. This variation pushes characters.
 * Characters are only valid within the ranges smaller than 0x10ffff and not
 * within 0xD800 to 0xDFFF (inclusive).
 *
 * If the pushed value is *not* within a valid character range, this function
 * returns `false` and nothing will be pushed onto the stack.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
bool rune_stack_push_char(rune_stack *stack, uint32_t value);

/**
 * Push a tuple with `count` elements onto the stack. The components of the
 * tuple will be popped from the stack in the reverse order that they were
 * pushed.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
bool rune_stack_push_tuple(rune_stack *stack, uintptr_t count, rune_vm_error *error);

/**
 * Push a vector with `count` elements onto the stack. The elements of the
 * vector will be popped from the stack in the reverse order that they were
 * pushed.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut].
 */
bool rune_stack_push_vec(rune_stack *stack, uintptr_t count, rune_vm_error *error);

/**
 * Pop an integer from the stack.
 *
 * Return a boolean indicating if a value was popped off the stack. The value
 * is only populated if the popped value matched the given value.
 *
 * # Safety
 *
 * Must be called with a valid stack. Like one fetched from
 * [rune_vm_stack_mut][crate:rune_vm_stack_mut]. The `value` must also have
 * been allocated correctly.
 */
bool rune_stack_pop_value(rune_stack *stack, rune_value *value, rune_vm_error *error);

/**
 * Construct a new empty unit handle.
 */
rune_unit rune_unit_new(void);

/**
 * Free a unit. Calling this multiple times on the same handle is allowed.
 *
 * This is a reference counted object. If the reference counts goes to 0, the
 * underlying object is freed.
 *
 * # Safety
 *
 * The `unit` argument must have been allocated with [rune_unit_new].
 */
void rune_unit_free(rune_unit *unit);

/**
 * Clone the given unit and return a new handle. Cloning increases the
 * reference count of the unit by one.
 *
 * # Safety
 *
 * The `unit` argument must have been allocated with [rune_unit_new].
 */
rune_unit rune_unit_clone(const rune_unit *unit);

/**
 * Construct a new context.
 */
rune_module rune_module_new(void);

/**
 * Free the given module.
 *
 * # Safety
 *
 * The `module` argument must have been allocated with [rune_module_new].
 */
void rune_module_free(rune_module *module);

/**
 * Register a toplevel function to the module.
 *
 * Returns `false` if the module is freed or the name is not valid UTF-8.
 *
 * # Safety
 *
 * The `module` argument must have been allocated with [rune_module_new] and
 * `name` must be a NULL-terminated string.
 */
bool rune_module_function(rune_module *module,
                          const char *name,
                          Function function,
                          rune_context_error *error);

/**
 * Allocate an empty runtime context.
 */
rune_runtime_context rune_runtime_context_new(void);

/**
 * Free the given runtime context.
 *
 * This is a reference counted object. If the reference counts goes to 0, the
 * underlying object is freed.
 *
 * # Safety
 *
 * Function must be called with a `runtime` argument that has been allocated by
 * [rune_runtime_context_new].
 */
void rune_runtime_context_free(rune_runtime_context *runtime);

/**
 * Clone the given runtime context and return a new reference.
 *
 * # Safety
 *
 * Function must be called with a `runtime` argument that has been allocated by
 * [rune_runtime_context_new].
 */
rune_runtime_context rune_runtime_context_clone(const rune_runtime_context *runtime);

/**
 * Construct a unit value.
 *
 * Even though not strictly necessary, it is good practice to always free your
 * values with [rune_value_free].
 *
 * \code{.c}
 * int main() {
 *     rune_vm_value value = rune_value_unit();
 *
 *     // ...
 *
 *     rune_value_free(&value);
 * }
 * \endcode
 */
rune_value rune_value_unit(void);

/**
 * Get the type hash of a value. Getting the type hash might error in case the
 * value is no longer accessible. If this happens, the empty hash is returned
 * and `error` is populated with the error that occured.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such as
 * [rune_value_unit] and a valid `error`.
 */
bool rune_value_type_hash(const rune_value *value, rune_hash *output, rune_vm_error *error);

/**
 * Simplified accessor for the type hash of the value which returns an
 * [rune_hash_empty][crate::rune_hash_empty] in case the type hash couldn't be
 * accessed and ignores the error.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such as
 * [rune_value_unit]`.
 */
rune_hash rune_value_type_hash_or_empty(const rune_value *value);

/**
 * Construct a value of the given type.
 */
rune_value rune_value_bool(bool value);

/**
 * Construct a value of the given type.
 */
rune_value rune_value_byte(uint8_t value);

/**
 * Construct a value of the given type.
 */
rune_value rune_value_integer(int64_t value);

/**
 * Construct a value of the given type.
 */
rune_value rune_value_float(double value);

/**
 * Construct a value of the given type.
 */
rune_value rune_value_type(rune_hash value);

/**
 * Construct a character value.
 *
 * Characters are only valid within the ranges smaller than 0x10ffff and not
 * within 0xD800 to 0xDFFF (inclusive).
 *
 * If the pushed value is *not* within a valid character range, this function
 * returns `false`.
 *
 * # Safety
 *
 * The caller must ensure that `output` is allocated using something like
 * [rune_value_unit].
 */
bool rune_value_char(uint32_t value, rune_value *output);

/**
 * Free the given value.
 *
 * Strictly speaking, values which are Copy do not need to be freed, but you
 * should make a habit of freeing any value used anywhere.
 *
 * This function is a little bit smart and sets the value to `Value::Unit` in
 * order to free it. This mitigates that subsequent calls to `rn_value_free`
 * doubly frees any allocated data.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such as
 * [rune_value_unit].
 */
void rune_value_free(rune_value *value);

/**
 * Set the value to the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
void rune_value_set_bool(rune_value *value, bool input);

/**
 * Set the value to the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
void rune_value_set_byte(rune_value *value, uint8_t input);

/**
 * Set the value to the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
void rune_value_set_char(rune_value *value, uint32_t input);

/**
 * Set the value to the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
void rune_value_set_integer(rune_value *value, int64_t input);

/**
 * Set the value to the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
void rune_value_set_float(rune_value *value, double input);

/**
 * Set the value to the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
void rune_value_set_type(rune_value *value, rune_hash input);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_unit(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_bool(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_byte(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_char(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_integer(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_float(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_type(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_string(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_bytes(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_vec(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_tuple(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_object(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_range(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_future(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_stream(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_generator(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_generatorstate(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_option(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_result(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_unitstruct(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_tuplestruct(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_struct(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_variant(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_function(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_format(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_iterator(const rune_value *value);

/**
 * Test if the value is of the given type.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_is_any(const rune_value *value);

/**
 * Coerce value into the given type. If the coercion was successful
 * returns `true` and consumed the value.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_as_bool(const rune_value *value, bool *output);

/**
 * Coerce value into the given type. If the coercion was successful
 * returns `true` and consumed the value.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_as_byte(const rune_value *value, uint8_t *output);

/**
 * Coerce value into the given type. If the coercion was successful
 * returns `true` and consumed the value.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_as_char(const rune_value *value, uint32_t *output);

/**
 * Coerce value into the given type. If the coercion was successful
 * returns `true` and consumed the value.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_as_integer(const rune_value *value, int64_t *output);

/**
 * Coerce value into the given type. If the coercion was successful
 * returns `true` and consumed the value.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_as_float(const rune_value *value, double *output);

/**
 * Coerce value into the given type. If the coercion was successful
 * returns `true` and consumed the value.
 *
 * # Safety
 *
 * The `value` argument must have been allocated with a function such
 * as [rune_value_unit].
 */
bool rune_value_as_type(const rune_value *value, rune_hash *output);

/**
 * Allocate space for a virtual machine.
 */
rune_vm rune_vm_new(void);

/**
 * Set up new virtual machine and assign it to `vm`.
 *
 * This takes ownership of the passed in `unit` and `runtime`. If either the
 * `runtime` or `unit` is not set this function will return `false`.
 *
 * # Safety
 *
 * Must be called with a `vm` that has been allocated with [rune_vm_new] and a
 * valid `runtime` and `unit` argument.
 */
bool rune_vm_setup(rune_vm *vm, rune_runtime_context *runtime, rune_unit *unit);

/**
 * Run the virtual machine to completion.
 *
 * This will replace `value`, freeing any old value which is already in place.
 *
 * Returns `true` if the virtual machine was successfully run to completion.
 *
 * # Safety
 *
 * Must be called with a `vm` that has been allocated with [rune_vm_new] and a
 * valid `value` and `error` argument.
 */
bool rune_vm_complete(rune_vm *vm, rune_value *value, rune_vm_error *error);

/**
 * Set the entrypoint to the given hash in the virtual machine.
 *
 * # Safety
 *
 * Must be called with a `vm` that has been allocated with [rune_vm_new] and a
 * valid `error` argument.
 */
bool rune_vm_set_entrypoint(rune_vm *vm, rune_hash hash, uintptr_t args, rune_vm_error *error);

/**
 * Get the stack associated with the virtual machine. If `vm` is not set returns NULL.
 *
 * # Safety
 *
 * Must be called with a `vm` that has been allocated with [rune_vm_new].
 */
rune_stack *rune_vm_stack_mut(rune_vm *vm);

/**
 * Free a virtual machine.
 *
 * # Safety
 *
 * Must be called with a `vm` that has been allocated with [rune_vm_new].
 */
void rune_vm_free(rune_vm *vm);

/**
 * Construct an empty [VmError].
 */
rune_vm_error rune_vm_error_new(void);

/**
 * Free the given virtual machine error.
 *
 * # Safety
 *
 * Must be called with an error that has been allocated with
 * [rune_vm_error_new].
 */
void rune_vm_error_free(rune_vm_error *error);

/**
 * Emit diagnostics to the given stream if the error is set. If the error is
 * not set nothing will be emitted.
 *
 * TODO: propagate I/O errors somehow.
 *
 * # Safety
 *
 * Must be called with an error that has been allocated with
 * [rune_vm_error_new].
 */
bool rune_vm_error_emit(const rune_vm_error *error,
                        rune_standard_stream *stream,
                        const rune_sources *sources);

/**
 * Set the given error to report a bad argument count error where the `actual`
 * number of arguments were provided instead of `expected`.
 *
 * This will replace any errors already reported.
 *
 * # Safety
 *
 * Must be called with an error that has been allocated with
 * [rune_vm_error_new].
 */
void rune_vm_error_bad_argument_count(rune_vm_error *error, uintptr_t actual, uintptr_t expected);

/**
 * Set the given error to report a bad argument at the given position, which
 * did not have the `expected` type.
 *
 * This will replace any errors already reported.
 *
 * # Safety
 *
 * Must be called with an error that has been allocated with
 * [rune_vm_error_new].
 */
void rune_vm_error_bad_argument_at(rune_vm_error *error,
                                   uintptr_t arg,
                                   const rune_value *actual,
                                   rune_static_type expected);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* RUNE_H */
