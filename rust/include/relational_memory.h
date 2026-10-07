#ifndef RELATIONAL_MEMORY_H
#define RELATIONAL_MEMORY_H
#include <stddef.h>
#include <stdint.h>
/* Standalone learning ABI, NOT Vexspoke Memory_* compatibility.
 * Owner is opaque and exclusively managed; drop once, never concurrently.
 * IDs are owner-local. Pointer arguments must be live, aligned and sized;
 * null source is valid only for zero length, null destination for empty data.
 * Output parameters must not alias inputs/owner. Failures preserve outputs.
 * Status: 0 OK, 1 invalid, 2 allocation/exhaustion, 3 unknown ID, 4 capacity.
 * new may abort on allocator OOM. No per-frame/real-time guarantees. */
typedef struct ReMemory ReMemory;
ReMemory *re_memory_new(void);
void re_memory_drop(ReMemory *owner);
uint32_t re_memory_copy(ReMemory *owner, const uint8_t *source, size_t length, uint64_t *output);
uint32_t re_memory_read(const ReMemory *owner, uint64_t id, uint8_t *destination, size_t capacity, size_t *output_length);
#endif
