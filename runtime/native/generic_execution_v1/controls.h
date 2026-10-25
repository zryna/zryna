
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
