static int vec_layout(uint32_t id, uint64_t *stride, uint32_t *alignment) {
  (void)stride;
  (void)alignment;
  switch (id) {
  /* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */
  default:
    return 0;
  }
}

static int valid_vec_storage(const zryna_rt_o1_handle *storage,
                             uint64_t stride, uint32_t alignment) {
  allocation_header *header;
  if (storage == NULL || stride == 0 || storage->length > storage->capacity ||
      storage->capacity > MAX_VEC_ELEMENTS ||
      storage->capacity > MAX_DYNAMIC_ALLOCATION_BYTES / stride) {
    return 0;
  }
  if (storage->capacity == 0) {
    return storage->pointer == 0;
  }
  header = header_for(storage->pointer);
  return header != NULL && header->size == storage->capacity * stride &&
         header->alignment == alignment && header->pending_last_strong == 0;
}

uint32_t zryna_rt_o1_vec_allocate(uint32_t element_layout_id,
                                  uint64_t required_capacity,
                                  zryna_rt_o1_handle *out_storage) {
  uint64_t stride;
  uint32_t alignment;
  uintptr_t pointer;
  uint32_t status;
  if (out_storage == NULL) {
    return RT_ABI;
  }
  memset(out_storage, 0, sizeof(*out_storage));
  if (!vec_layout(element_layout_id, &stride, &alignment)) {
    return RT_ABI;
  }
  if (required_capacity > MAX_VEC_ELEMENTS ||
      required_capacity > MAX_DYNAMIC_ALLOCATION_BYTES / stride) {
    return RT_CAPACITY;
  }
  status = allocate_bytes(required_capacity * stride, alignment, &pointer);
  if (status != RT_OK) {
    return status;
  }
  out_storage->pointer = pointer;
  out_storage->capacity = required_capacity;
  return RT_OK;
}

uint32_t zryna_rt_o1_vec_reserve(uint32_t element_layout_id,
                                 const zryna_rt_o1_handle *storage,
                                 uint64_t required_length,
                                 zryna_rt_o1_handle *out_storage) {
  uint64_t stride;
  uint64_t capacity;
  uint32_t alignment;
  uintptr_t pointer;
  uint32_t status;
  if (storage == out_storage) {
    return RT_ABI;
  }
  if (out_storage == NULL || !vec_layout(element_layout_id, &stride, &alignment) ||
      !valid_vec_storage(storage, stride, alignment) ||
      required_length < storage->length) {
    if (out_storage != NULL) {
      memset(out_storage, 0, sizeof(*out_storage));
    }
    return RT_ABI;
  }
  memset(out_storage, 0, sizeof(*out_storage));
  if (required_length <= storage->capacity) {
    *out_storage = *storage;
    return RT_OK;
  }
  capacity = storage->capacity == 0 ? 1 : storage->capacity;
  while (capacity < required_length && capacity <= MAX_VEC_ELEMENTS / 2) {
    capacity *= 2;
  }
  if (capacity < required_length) {
    capacity = required_length;
  }
  if (capacity > MAX_VEC_ELEMENTS || capacity > MAX_DYNAMIC_ALLOCATION_BYTES / stride) {
    return RT_CAPACITY;
  }
  status = zryna_rt_o1_grow(storage->pointer, storage->capacity * stride,
                            capacity * stride, alignment, &pointer);
  if (status != RT_OK) {
    return status;
  }
  out_storage->pointer = pointer;
  out_storage->length = storage->length;
  out_storage->capacity = capacity;
  return RT_OK;
}

uint32_t zryna_rt_o1_vec_release_storage(uint32_t element_layout_id,
                                         const zryna_rt_o1_handle *storage) {
  uint64_t stride;
  uint32_t alignment;
  if (!vec_layout(element_layout_id, &stride, &alignment) ||
      !valid_vec_storage(storage, stride, alignment)) {
    return RT_ABI;
  }
  return zryna_rt_o1_release(storage->pointer, storage->capacity * stride,
                             alignment);
}

uint32_t zryna_rt_o1_strong_clone(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *strong = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0 ||
      *strong == 0) {
    return RT_ABI;
  }
  if (*strong == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++*strong;
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_downgrade(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0 ||
      counts[0] == 0) {
    return RT_ABI;
  }
  if (counts[1] == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++counts[1];
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_clone(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0 ||
      counts[1] == 0) {
    return RT_ABI;
  }
  if (counts[1] == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++counts[1];
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_upgrade(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *strong = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0) {
    return RT_ABI;
  }
  if (*strong == 0) {
    return RT_EXPIRED;
  }
  if (*strong == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++*strong;
  return RT_OK;
}

uint32_t zryna_rt_o1_strong_release_begin(uintptr_t control,
                                          uint32_t *out_is_last_strong) {
  allocation_header *header = header_for(control);
  uint32_t *strong = (uint32_t *)control;
  if (out_is_last_strong == NULL || !valid_control_storage(header) ||
      header->pending_last_strong != 0 || *strong == 0) {
    return RT_ABI;
  }
  --*strong;
  *out_is_last_strong = (uint32_t)(*strong == 0);
  header->pending_last_strong = *out_is_last_strong;
  return RT_OK;
}

uint32_t zryna_rt_o1_strong_release_finish(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong == 0 ||
      counts[0] != 0 || counts[1] == 0) {
    return RT_ABI;
  }
  --counts[1];
  header->pending_last_strong = 0;
  if (counts[1] == 0) {
    return release_known(header);
  }
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_release(uintptr_t control,
                                  uint32_t *out_deallocated) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (out_deallocated == NULL || !valid_control_storage(header) ||
      header->pending_last_strong != 0 || counts[1] == 0 ||
      (counts[0] != 0 && counts[1] == 1)) {
    return RT_ABI;
  }
  --counts[1];
  *out_deallocated = (uint32_t)(counts[1] == 0);
  if (*out_deallocated != 0) {
    return release_known(header);
  }
  return RT_OK;
}

#ifdef ZRYNA_M3_OBSERVATION
/* Non-owning SSA descriptors live until the scalar invocation has completed. */
uint32_t zryna_m3_allocate_record(uint64_t size, uint32_t alignment, uintptr_t *out) {
  uint32_t status = allocate_bytes(size, alignment, out);
  if (status == RT_OK && *out != 0) {
    allocation_header *header = header_for(*out);
    if (header == NULL) return RT_ABI;
    header->scratch = 1;
  }
  return status;
}
uint32_t zryna_m3_finish_invocation(void) {
  allocation_header *current = allocation_head;
  while (current != NULL) {
    allocation_header *next = current->next;
    if (current->scratch && release_known(current) != RT_OK) return RT_ABI;
    current = next;
  }
  /* Owned payloads/control blocks must have been released by generated cleanup. */
  return allocation_head == NULL ? RT_OK : RT_ABI;
}
#endif
