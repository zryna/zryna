#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* All large admitted requests fail before malloc; no large payload is created. */
#define ZRYNA_RT_O1_FAIL_ALLOCATION_AT 1
#include "../../../runtime/native/ownership_runtime_v1.c"

int main(void) {
  static const uint64_t sizes[] = {
      UINT64_C(67108863), UINT64_C(67108864), UINT64_C(67108865),
      UINT64_C(2147483647), UINT64_C(2147483648), UINT64_MAX};
  static const uint32_t expected[] = {
      RT_ALLOCATION, RT_ALLOCATION, RT_ALLOCATION,
      RT_ALLOCATION, RT_CAPACITY, RT_CAPACITY};
  const uint8_t byte = 'x';
  zryna_rt_o1_handle string = {99, 99, 99};
  uintptr_t pointer = 99;
  uintptr_t grown = 99;
  for (size_t index = 0; index < sizeof(sizes) / sizeof(sizes[0]); index++) {
    allocation_attempt = 0;
    pointer = 99;
    assert(zryna_rt_o1_allocate(sizes[index], 1, &pointer) == expected[index]);
    assert(pointer == 0 && allocation_head == NULL);
  }
  assert(zryna_rt_o1_allocate(8, 3, &pointer) == RT_ABI);
  assert(pointer == 0 && allocation_head == NULL);
  assert(zryna_rt_o1_string_from_utf8_copy(&byte, UINT64_C(2147483648), &string) ==
         RT_CAPACITY);
  assert(string.pointer == 0 && string.length == 0 && string.capacity == 0);

  allocation_attempt = 1;
  assert(zryna_rt_o1_allocate(8, 8, &pointer) == RT_OK && pointer != 0);
  memset((void *)pointer, 0x5a, 8);
  allocation_attempt = 0;
  assert(zryna_rt_o1_grow(pointer, 8, UINT64_C(67108865), 8, &grown) == RT_ALLOCATION);
  assert(grown == 0 && ((uint8_t *)pointer)[0] == 0x5a);
  assert(zryna_rt_o1_release(pointer, 8, 8) == RT_OK && allocation_head == NULL);

  allocation_attempt = 1;
  assert(zryna_rt_o1_allocate(1, 1, &pointer) == RT_OK && pointer != 0);
  assert(zryna_rt_o1_release(pointer, 1, 1) == RT_OK && allocation_head == NULL);
  puts("native capacity observation passed");
  return 0;
}
