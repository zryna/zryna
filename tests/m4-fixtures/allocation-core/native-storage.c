#include <assert.h>
#include <stdio.h>

static void string_bytes(const zryna_rt_o1_handle *value,
                         const uint8_t *bytes, size_t length) {
  assert(value->length == length);
  assert(value->capacity >= length);
  assert(memcmp((const void *)value->pointer, bytes, length) == 0);
}

static void zero_result(const zryna_rt_o1_handle *value) {
  assert(value->pointer == 0 && value->length == 0 && value->capacity == 0);
}

int main(void) {
  static const uint8_t text[] = {104, 195, 169};
  static const uint8_t suffix[] = {33};
  static const uint8_t joined[] = {104, 195, 169, 33};
  zryna_rt_o1_handle source = {0}, copied = {0}, right = {0}, result = {0};
  zryna_rt_o1_handle vector = {0}, replacement = {0};
  uint64_t attempts;

  fail_at = allocation_attempt + 1;
  assert(zryna_rt_o1_string_from_utf8_copy(text, sizeof(text), &source) == RT_ALLOCATION);
  zero_result(&source);
  assert(allocation_head == NULL);
  fail_at = 0;
  assert(zryna_rt_o1_string_from_utf8_copy(text, sizeof(text), &source) == RT_OK);
  assert(zryna_rt_o1_string_clone(&source, &copied) == RT_OK);
  assert(source.pointer != copied.pointer);
  string_bytes(&source, text, sizeof(text));
  string_bytes(&copied, text, sizeof(text));
  assert(zryna_rt_o1_string_from_utf8_copy(suffix, sizeof(suffix), &right) == RT_OK);
  assert(zryna_rt_o1_string_concat(&source, &right, &result) == RT_OK);
  assert(result.pointer != source.pointer && result.pointer != right.pointer);
  string_bytes(&result, joined, sizeof(joined));
  assert(zryna_rt_o1_string_release(&result) == RT_OK);
  string_bytes(&source, text, sizeof(text));
  string_bytes(&right, suffix, sizeof(suffix));

  fail_at = allocation_attempt + 1;
  assert(zryna_rt_o1_string_clone(&source, &result) == RT_ALLOCATION);
  zero_result(&result);
  string_bytes(&source, text, sizeof(text));
  fail_at = allocation_attempt + 1;
  assert(zryna_rt_o1_string_concat(&source, &right, &result) == RT_ALLOCATION);
  zero_result(&result);
  string_bytes(&source, text, sizeof(text));
  string_bytes(&right, suffix, sizeof(suffix));
  assert(zryna_rt_o1_string_release(&right) == RT_OK);
  assert(zryna_rt_o1_string_release(&copied) == RT_OK);
  assert(zryna_rt_o1_string_release(&source) == RT_OK);
  assert(allocation_head == NULL);

  fail_at = 0;
  assert(zryna_rt_o1_vec_allocate(7, 2, &vector) == RT_OK);
  ((int32_t *)vector.pointer)[0] = 7;
  ((int32_t *)vector.pointer)[1] = 9;
  vector.length = 2;
  assert(vector.capacity == 2);
  fail_at = allocation_attempt + 1;
  assert(zryna_rt_o1_vec_reserve(7, &vector, 3, &replacement) == RT_ALLOCATION);
  zero_result(&replacement);
  assert(vector.length == 2 && vector.capacity == 2);
  assert(((int32_t *)vector.pointer)[0] == 7 && ((int32_t *)vector.pointer)[1] == 9);
  fail_at = 0;
  assert(zryna_rt_o1_vec_reserve(7, &vector, 3, &replacement) == RT_OK);
  assert(replacement.capacity == 4 && replacement.length == 2);
  assert(((int32_t *)replacement.pointer)[0] == 7 && ((int32_t *)replacement.pointer)[1] == 9);
  ((int32_t *)replacement.pointer)[2] = 13;
  replacement.length = 3;
  assert(((int32_t *)replacement.pointer)[2] == 13);
  assert(zryna_rt_o1_vec_release_storage(7, &replacement) == RT_OK);
  assert(allocation_head == NULL);

  /* Boundaries use deterministic allocation failure, never host exhaustion. */
  fail_at = allocation_attempt + 1;
  assert(zryna_rt_o1_vec_allocate(7, 1048576, &result) == RT_ALLOCATION);
  zero_result(&result);
  attempts = allocation_attempt;
  assert(zryna_rt_o1_vec_allocate(7, 1048577, &result) == RT_CAPACITY);
  assert(allocation_attempt == attempts);
  zero_result(&result);
  assert(allocation_head == NULL);
  puts("native allocation observation passed");
  return 0;
}
