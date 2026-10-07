#ifndef RELATIONAL_ENGINE_VARIABLE_REGISTRY_H
#define RELATIONAL_ENGINE_VARIABLE_REGISTRY_H
#include <stddef.h>
#include <stdint.h>

// One opaque owner plus its behaviorless borrowed row view. Rust owns all rows.
typedef struct ReVariableRegistry ReVariableRegistry;
typedef struct ReVariableSlot {
    uint8_t name[24];
    const uint8_t *pointer; // Borrowed value address, NOT the slot's own address.
} ReVariableSlot;
static_assert(sizeof(ReVariableSlot) == 32, "64-bit variable slot ABI");
static_assert(offsetof(ReVariableSlot, name) == 0, "name starts at zero");
static_assert(offsetof(ReVariableSlot, pointer) == 24, "value pointer follows name");

// Status: 0 success; 1 invalid input/layout/name; 2 allocation/exhaustion;
// 3 absent/index bounds; 4 duplicate. Errors preserve outputs and existing rows.
// All calls require external exclusion of mutation/drop; no shared mutation claim.
// Non-null pointers must be valid/live/aligned and outputs disjoint from inputs
// and owner. Arbitrary/stale pointer validity is NOT checked. Value pointers are
// opaque and may be null; no function dereferences or frees the pointed value.
// Slot addresses survive growth and remain borrowed until owner drop. A returned
// view must not be mutated; exclude all readers before rebinding or destruction.
// Name bytes are 1..23 ASCII bytes, folded to lowercase; dots split nonempty
// segments of [a-z0-9_$-]. Embedded NUL/non-ASCII reject. No silent truncation.
// Registry owner Box construction may abort on allocator OOM; chunk/directory
// allocation uses fallible allocation. No schema migration or default allocator change.
uint32_t re_variables_new(size_t rowsPerChunk, ReVariableRegistry **outOwner);
void re_variables_drop(ReVariableRegistry *owner);
uint32_t re_variables_add(ReVariableRegistry *owner, const uint8_t *name,
    size_t nameBytes, const uint8_t *valuePointer, size_t *outIndex);
uint32_t re_variables_find(const ReVariableRegistry *owner, const uint8_t *name,
    size_t nameBytes, size_t *outIndex);
uint32_t re_variables_slot(const ReVariableRegistry *owner, size_t index,
    const ReVariableSlot **outSlot);
uint32_t re_variables_set_pointer(ReVariableRegistry *owner, size_t index,
    const uint8_t *valuePointer);
#endif
