/* Raw canonical-runtime owner controls, not source/IR/profile qualification.
 * Compile this translation unit alone: it includes the private generic runtime.
 * Both default and -DZRYNA_M3_OBSERVATION=1 configurations are required. */
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static size_t malloc_calls, free_calls, last_malloc_bytes;
static int fail_next_malloc;
static uintptr_t last_freed[8];
static void *registry_test_malloc(size_t bytes) {
  ++malloc_calls;
  last_malloc_bytes = bytes;
  if (fail_next_malloc) { fail_next_malloc = 0; return NULL; }
  return malloc(bytes);
}
static void registry_test_free(void *pointer) {
  if (free_calls < 8) last_freed[free_calls] = (uintptr_t)pointer;
  ++free_calls;
  free(pointer);
}
#define malloc registry_test_malloc
#define free registry_test_free
#include "../../runtime/native/generic_execution_v1.c"
#undef malloc
#undef free

/* Freeze physical geometry separately from the registry representation. */
typedef struct legacy_allocation_header_tag {
  uint64_t magic, size;
  uintptr_t base, data;
  struct legacy_allocation_header_tag *previous, *next;
  uint32_t alignment, pending_last_strong;
#ifdef ZRYNA_M3_OBSERVATION
  uint32_t scratch;
#endif
} legacy_allocation_header;
_Static_assert(sizeof(allocation_header) == sizeof(legacy_allocation_header),
               "registry must not change physical allocation size");
#define SAME_OFFSET(field) \
  _Static_assert(offsetof(allocation_header, field) == \
                 offsetof(legacy_allocation_header, field), "header offset changed")
SAME_OFFSET(magic); SAME_OFFSET(size); SAME_OFFSET(base); SAME_OFFSET(data);
SAME_OFFSET(previous); SAME_OFFSET(next); SAME_OFFSET(alignment);
SAME_OFFSET(pending_last_strong);
#ifdef ZRYNA_M3_OBSERVATION
SAME_OFFSET(scratch);
static size_t observed_attempts;
uint32_t zryna_m3_observe(uint32_t command) {
  assert(command == UINT32_C(0x10000006));
  ++observed_attempts;
  return 0;
}
#endif
#undef SAME_OFFSET

static void empty_registry(void) {
#ifdef ZRYNA_M3_OBSERVATION
  assert(allocation_head == NULL);
#else
  for (size_t i = 0; i < ALLOCATION_BUCKETS; ++i) assert(allocation_buckets[i] == NULL);
#endif
}

static void invalid_and_growth(void) {
  uintptr_t pointer = 0, replacement = 0;
  size_t calls;
  uint64_t foreign = UINT64_C(0x0102030405060708);
  assert(zryna_rt_o1_release(0, 0, 8) == RT_OK);
  assert(zryna_rt_o1_release(0, 1, 8) == RT_ABI);
  assert(zryna_rt_o1_release((uintptr_t)&foreign, sizeof(foreign), 8) == RT_ABI);
  assert(zryna_rt_o1_release(UINTPTR_MAX, 1, 1) == RT_ABI);
  assert(zryna_rt_o1_strong_clone(UINTPTR_MAX) == RT_ABI);
  assert(zryna_rt_o1_allocate(32, 8, &pointer) == RT_OK);
  assert(last_malloc_bytes == 32 + sizeof(legacy_allocation_header) + 7);
  for (size_t i = 0; i < 32; ++i) ((unsigned char *)pointer)[i] = (unsigned char)(i + 1);
  calls = malloc_calls;
  assert(zryna_rt_o1_release(pointer + 1, 31, 1) == RT_ABI);
  assert(zryna_rt_o1_release(pointer, 31, 8) == RT_ABI);
  assert(zryna_rt_o1_release(pointer, 32, 4) == RT_ABI);
  assert(malloc_calls == calls);
  fail_next_malloc = 1;
  replacement = UINTPTR_MAX;
  assert(zryna_rt_o1_grow(pointer, 32, 64, 8, &replacement) == RT_ALLOCATION);
  assert(replacement == 0);
  for (size_t i = 0; i < 32; ++i) assert(((unsigned char *)pointer)[i] == i + 1);
  assert(zryna_rt_o1_grow(pointer, 32, 64, 8, &replacement) == RT_OK);
  assert(replacement != pointer);
  assert(zryna_rt_o1_release(pointer, 32, 8) == RT_ABI);
  for (size_t i = 0; i < 32; ++i) assert(((unsigned char *)replacement)[i] == i + 1);
  pointer = replacement;
  calls = malloc_calls;
  assert(zryna_rt_o1_grow(pointer, 64, 64, 8, &replacement) == RT_OK);
  assert(replacement == pointer && malloc_calls == calls);
  assert(zryna_rt_o1_grow(pointer, 64, 16, 8, &replacement) == RT_OK);
  assert(zryna_rt_o1_release(pointer, 64, 8) == RT_ABI);
  for (size_t i = 0; i < 16; ++i) assert(((unsigned char *)replacement)[i] == i + 1);
  pointer = replacement;
  calls = malloc_calls;
  assert(zryna_rt_o1_grow(pointer, 16, 0, 8, &replacement) == RT_OK);
  assert(replacement == 0 && malloc_calls == calls);
  assert(zryna_rt_o1_release(pointer, 16, 8) == RT_ABI);
  assert(zryna_rt_o1_allocate(32, 8, &pointer) == RT_OK);
  calls = malloc_calls;
  assert(zryna_rt_o1_release(pointer, 32, 8) == RT_OK);
  assert(malloc_calls == calls);
  assert(zryna_rt_o1_release(pointer, 32, 8) == RT_ABI);
  empty_registry();
}

static void controls(void) {
  uintptr_t control = 0;
  uint32_t last = 99;
  size_t calls;
  assert(zryna_rt_o1_allocate(16, 4, &control) == RT_OK);
  ((uint32_t *)control)[0] = 1; ((uint32_t *)control)[1] = 2;
  assert(zryna_rt_o1_strong_clone(control) == RT_OK);
  assert(zryna_rt_o1_strong_release_begin(control, &last) == RT_OK && last == 0);
  assert(zryna_rt_o1_weak_clone(control) == RT_OK);
  assert(zryna_rt_o1_weak_release(control, &last) == RT_OK && last == 0);
  assert(zryna_rt_o1_strong_release_begin(control, &last) == RT_OK && last == 1);
  last = 99; calls = malloc_calls;
  assert(zryna_rt_o1_weak_release(control, &last) == RT_ABI && last == 99);
  assert(zryna_rt_o1_release(control, 16, 4) == RT_ABI);
  assert(zryna_rt_o1_strong_release_finish(control) == RT_OK);
  assert(zryna_rt_o1_weak_upgrade(control) == RT_EXPIRED);
  assert(zryna_rt_o1_weak_clone(control) == RT_OK);
  assert(zryna_rt_o1_weak_release(control, &last) == RT_OK && last == 0);
  assert(zryna_rt_o1_weak_release(control, &last) == RT_OK && last == 1);
  assert(malloc_calls == calls);
  assert(zryna_rt_o1_strong_clone(control) == RT_ABI);
  empty_registry();
}

#ifndef ZRYNA_M3_OBSERVATION
static void verify_chains(void) {
  for (size_t bucket = 0; bucket < ALLOCATION_BUCKETS; ++bucket) {
    allocation_header *previous = NULL;
    size_t length = 0;
    for (allocation_header *h = allocation_buckets[bucket]; h != NULL; h = h->next) {
      assert(++length <= 2 * ALLOCATION_BUCKETS + 1);
      assert(h->previous == previous && h->magic == ALLOCATION_MAGIC);
      assert(registry_head(h->data) == &allocation_buckets[bucket]);
      previous = h;
    }
  }
}
static void collisions_and_unlink(void) {
  enum { COUNT = 2 * ALLOCATION_BUCKETS + 1 };
  static uintptr_t pointers[COUNT];
  static uint64_t sizes[COUNT];
  uintptr_t first = 0, middle = 0, last = 0, result = 0;
  uint64_t first_size = 0, middle_size = 0, last_size = 0;
  size_t calls;
  for (size_t i = 0; i < COUNT; ++i) {
    assert(zryna_rt_o1_allocate(8, 8, &pointers[i]) == RT_OK);
    *(uint64_t *)pointers[i] = (uint64_t)i + 1;
    sizes[i] = 8;
  }
  verify_chains();
  /* Grow an interior collision member while all other owners stay live. */
  for (size_t i = 0; i < ALLOCATION_BUCKETS; ++i) {
    allocation_header *head = allocation_buckets[i];
    if (head != NULL && head->next != NULL && head->next->next != NULL) {
      uintptr_t old = head->next->data;
      size_t index = (size_t)(*(uint64_t *)old - 1U);
      assert(index < COUNT && pointers[index] == old);
      calls = malloc_calls;
      assert(zryna_rt_o1_grow(old, 8, 16, 8, &result) == RT_OK);
      assert(malloc_calls == calls + 1 && result != old);
      assert(*(uint64_t *)result == (uint64_t)index + 1);
      assert(zryna_rt_o1_release(old, 8, 8) == RT_ABI);
      pointers[index] = result; sizes[index] = 16;
      break;
    }
  }
  verify_chains();
  for (size_t i = 0; i < ALLOCATION_BUCKETS; ++i) {
    allocation_header *head = allocation_buckets[i];
    if (head != NULL && head->next != NULL && head->next->next != NULL) {
      first = head->data; first_size = head->size;
      middle = head->next->data; middle_size = head->next->size;
      while (head->next != NULL) head = head->next;
      last = head->data; last_size = head->size; break;
    }
  }
  assert(first != 0 && middle != 0 && last != 0);
  assert(first != middle && middle != last && first != last);
  calls = malloc_calls;
  assert(zryna_rt_o1_release(middle, middle_size, 8) == RT_OK);
  verify_chains();
  assert(zryna_rt_o1_release(last, last_size, 8) == RT_OK);
  verify_chains();
  assert(zryna_rt_o1_release(first, first_size, 8) == RT_OK);
  verify_chains();
  assert(malloc_calls == calls);
  for (size_t i = 0; i < COUNT; ++i) {
    uintptr_t pointer = pointers[i];
    if (pointer == first || pointer == middle || pointer == last) {
      assert(zryna_rt_o1_release(pointer, sizes[i], 8) == RT_ABI);
      continue;
    }
    assert(*(uint64_t *)pointer == (uint64_t)i + 1);
    assert(zryna_rt_o1_grow(pointer, sizes[i], sizes[i], 8, &result) == RT_OK && result == pointer);
    assert(zryna_rt_o1_release(pointer, sizes[i], 8) == RT_OK);
  }
  assert(malloc_calls == calls);
  empty_registry();
}
#else
static void scratch_order_and_leak_detection(void) {
  uintptr_t one = 0, two = 0, three = 0, owned = 0;
  uintptr_t bases[3];
  size_t calls, frees;
  assert(zryna_m3_allocate_record(20, 8, &one) == RT_OK);
  assert(zryna_rt_o1_allocate(8, 8, &owned) == RT_OK);
  assert(zryna_m3_allocate_record(24, 8, &two) == RT_OK);
  assert(zryna_m3_allocate_record(28, 8, &three) == RT_OK);
  bases[0] = header_for(three)->base;
  bases[1] = header_for(two)->base;
  bases[2] = header_for(one)->base;
  calls = malloc_calls; frees = free_calls;
  assert(zryna_m3_finish_invocation() == RT_ABI);
  assert(malloc_calls == calls && free_calls == frees + 3);
  for (size_t i = 0; i < 3; ++i) assert(last_freed[frees + i] == bases[i]);
  assert(header_for(owned) != NULL);
  assert(zryna_rt_o1_release(owned, 8, 8) == RT_OK);
  assert(zryna_m3_finish_invocation() == RT_OK);
  empty_registry();
}
#endif

int main(void) {
  invalid_and_growth();
  controls();
#ifdef ZRYNA_M3_OBSERVATION
  /* Reset only observer bookkeeping, never the runtime's ownership registry. */
  free_calls = 0;
  scratch_order_and_leak_detection();
  assert(observed_attempts > 0);
#else
  collisions_and_unlink();
#endif
  empty_registry();
  puts("native registry controls passed");
  return 0;
}
