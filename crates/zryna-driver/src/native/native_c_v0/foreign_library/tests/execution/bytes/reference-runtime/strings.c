static int valid_utf8(const uint8_t *bytes, uint64_t length) {
  uint64_t index = 0;
  while (index < length) {
    uint8_t first = bytes[index++];
    uint32_t code;
    uint32_t minimum;
    uint32_t remaining;
    if (first < 0x80U) {
      continue;
    }
    if (first >= 0xc2U && first <= 0xdfU) {
      code = first & 0x1fU;
      minimum = 0x80U;
      remaining = 1;
    } else if (first >= 0xe0U && first <= 0xefU) {
      code = first & 0x0fU;
      minimum = 0x800U;
      remaining = 2;
    } else if (first >= 0xf0U && first <= 0xf4U) {
      code = first & 0x07U;
      minimum = 0x10000U;
      remaining = 3;
    } else {
      return 0;
    }
    if (length - index < remaining) {
      return 0;
    }
    while (remaining-- != 0) {
      uint8_t next = bytes[index++];
      if ((next & 0xc0U) != 0x80U) {
        return 0;
      }
      code = (code << 6) | (next & 0x3fU);
    }
    if (code < minimum || code > 0x10ffffU ||
        (code >= 0xd800U && code <= 0xdfffU)) {
      return 0;
    }
  }
  return 1;
}

uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,
                                            uint64_t byte_length,
                                            zryna_rt_o1_handle *out_string) {
  uintptr_t pointer;
  uint32_t status;
  if (out_string == NULL || (bytes == NULL && byte_length != 0)) {
    return RT_ABI;
  }
  memset(out_string, 0, sizeof(*out_string));
  if (byte_length > MAX_DYNAMIC_ALLOCATION_BYTES) return RT_CAPACITY;
  if (!valid_utf8(bytes, byte_length)) {
    return RT_UTF8;
  }
  status = allocate_bytes(byte_length, 1, &pointer);
  if (status != RT_OK) {
    return status;
  }
  if (byte_length != 0) {
    memcpy((void *)pointer, bytes, (size_t)byte_length);
  }
  out_string->pointer = pointer;
  out_string->length = byte_length;
  out_string->capacity = byte_length;
  return RT_OK;
}

uint32_t zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,
                                  zryna_rt_o1_handle *out_string) {
  if (out_string == source) {
    return RT_ABI;
  }
  if (!valid_string_storage(source)) {
    if (out_string != NULL) {
      memset(out_string, 0, sizeof(*out_string));
    }
    return RT_ABI;
  }
  return zryna_rt_o1_string_from_utf8_copy((const uint8_t *)source->pointer,
                                            source->length, out_string);
}

uint32_t zryna_rt_o1_string_concat(const zryna_rt_o1_handle *left,
                                   const zryna_rt_o1_handle *right,
                                   zryna_rt_o1_handle *out_string) {
  uint64_t length;
  uintptr_t pointer;
  uint32_t status;
  if (out_string == left || out_string == right) {
    return RT_ABI;
  }
  if (out_string == NULL || !valid_string_storage(left) ||
      !valid_string_storage(right)) {
    if (out_string != NULL) {
      memset(out_string, 0, sizeof(*out_string));
    }
    return RT_ABI;
  }
  memset(out_string, 0, sizeof(*out_string));
  if (left->length > MAX_DYNAMIC_ALLOCATION_BYTES - right->length) {
    return RT_CAPACITY;
  }
  length = left->length + right->length;
  status = allocate_bytes(length, 1, &pointer);
  if (status != RT_OK) {
    return status;
  }
  if (left->length != 0) {
    memcpy((void *)pointer, (const void *)left->pointer, (size_t)left->length);
  }
  if (right->length != 0) {
    memcpy((void *)(pointer + left->length), (const void *)right->pointer,
           (size_t)right->length);
  }
  out_string->pointer = pointer;
  out_string->length = length;
  out_string->capacity = length;
  return RT_OK;
}

uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  if (value == NULL || value->length > value->capacity) {
    return RT_ABI;
  }
  return zryna_rt_o1_release(value->pointer, value->capacity, 1);
}

