#ifndef RELATIONAL_MEMORY_H
#define RELATIONAL_MEMORY_H
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
/* Standalone learning ABI, NOT Vexspoke Memory_* compatibility.
 * Owner is opaque and exclusively managed; drop once, never concurrently.
 * IDs are owner-local. Pointer arguments must be live, aligned and sized;
 * null source is valid only for zero length, null destination for empty data.
 * Output parameters must not alias inputs/owner. Failures preserve outputs.
 * Status: 0 OK, 1 invalid, 2 allocation/exhaustion, 3 unknown ID, 4 capacity.
 * Atomic status extensions: 5 wrong kind, 6 byte index bounds, 7 writer busy.
 * Atomic get/set calls may overlap on an existing owner while registration,
 * release and destruction are excluded. Only atomic operations access atomics.
 * Strings publish whole immutable byte snapshots (not necessarily UTF-8).
 * Old snapshots stay until owner destruction; caller chooses retention budget.
 * String set may allocate, returns busy instead of waiting. Buffer-too-small
 * sets out_truncated=true without changing destination/output_length.
 * Engine code/storage must remain resident across consumer hot reloads.
 * new may abort on allocator OOM. No per-frame/real-time guarantees. */
typedef struct ReMemory ReMemory;
ReMemory *re_memory_new(void);
void re_memory_drop(ReMemory *owner);
uint32_t re_memory_copy(ReMemory *owner, const uint8_t *source, size_t length, uint64_t *output);
uint32_t re_memory_read(const ReMemory *owner, uint64_t id, uint8_t *destination, size_t capacity, size_t *output_length);
uint32_t re_memory_get_byte(const ReMemory *owner, uint64_t id, size_t index, uint8_t *output);
uint32_t re_memory_new_atomic_byte(ReMemory *owner, uint8_t value, uint64_t *output);
uint32_t re_memory_get_atomic_byte(const ReMemory *owner, uint64_t id, uint8_t *output);
uint32_t re_memory_set_atomic_byte(const ReMemory *owner, uint64_t id, uint8_t value);
uint32_t re_memory_new_atomic_string(ReMemory *owner, const uint8_t *source, size_t length, size_t retention_limit, uint64_t *output);
uint32_t re_memory_get_atomic_string(const ReMemory *owner, uint64_t id, uint8_t *destination, size_t capacity, size_t *output_length, bool *out_truncated);
uint32_t re_memory_set_atomic_string(const ReMemory *owner, uint64_t id, const uint8_t *source, size_t length);
#endif
