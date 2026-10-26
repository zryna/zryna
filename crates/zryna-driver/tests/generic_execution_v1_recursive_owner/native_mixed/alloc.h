/* Real canonical calls; physical malloc injection stays inside the unmodified C implementation. */
static void zero_output(const zryna_rt_o1_handle *out) { assert(out && !out->pointer && !out->length && !out->capacity); }
static void poison(zryna_rt_o1_handle *out) { zero_output(out);*out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX}; }
static void result(uint32_t status) { if(status)assert(status==1 && fault_hits==1); }
static uint32_t element_for(uint64_t id) {
  assert(id==2 || id==18 || id==19 || id==25);return id==2 || id==25 ? MIXED_NODE_ELEMENT : MIXED_STRING_ELEMENT;
}
uint32_t __wrap_zryna_rt_o1_allocate(uint64_t bytes,uint32_t alignment,uintptr_t *out) {
  assert(out && *out==0 && metadata_control(bytes,alignment));
  assert(bytes==MIXED_CONTROL_SIZE && alignment==MIXED_CONTROL_ALIGNMENT);
  unsigned previous=enter(0);uint32_t status=__real_zryna_rt_o1_allocate(bytes,alignment,out);result(status);
  if(status)assert(*out==0);else publish_storage(*out,bytes,3);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,uint64_t length,zryna_rt_o1_handle *out) {
  zero_output(out);assert(successes<26);const expected_row *row=&expected_rows[successes+1];
  assert(row->kind==1 && !row->source && length==row->length && memcmp(bytes,row->bytes,(size_t)length)==0);
  unsigned previous=enter(3);uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out);result(status);
  if(status) {zero_output(out);string_failure(2,fault_ordinal,0,bytes,length);}else publish_string(out,0);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,zryna_rt_o1_handle *out) {
  zero_output(out);(void)check_string(source);zryna_rt_o1_handle before=*source;
  unsigned previous=enter(4);string_clones++;
  if(fault_kind==6 && string_clones==fault_ordinal) {
    fault_hits++;event(6,4,string_clones,1);
    string_failure(6,fault_ordinal,source->pointer,(const uint8_t *)source->pointer,source->length);
    poison(out);assert(memcmp(source,&before,sizeof(before))==0);(void)check_string(source);
    leave(previous,1);return 1;
  }
  uint32_t status=__real_zryna_rt_o1_string_clone(source,out);result(status);
  assert(memcmp(source,&before,sizeof(before))==0);(void)check_string(source);
  if(status) {
    zero_output(out);string_failure(2,fault_ordinal,source->pointer,(const uint8_t *)source->pointer,source->length);
    poison(out); /* Hostile observer isolation, not canonical failed-output behavior. */
  } else {assert(out->pointer!=source->pointer);publish_string(out,source->pointer);}
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  unsigned previous=enter(6);release_string(value);
  uint32_t status=__real_zryna_rt_o1_string_release(value);assert(status==0);leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t element,uint64_t capacity,zryna_rt_o1_handle *out) {
  zero_output(out);assert(successes<26 && element==element_for(successes+1) && capacity==1);
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment));
  unsigned previous=enter(7);uint32_t status=__real_zryna_rt_o1_vec_allocate(element,capacity,out);result(status);
  if(status)zero_output(out);else {assert(out->length==0 && out->capacity==1);publish_storage(out->pointer,stride,2);}
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t element,const zryna_rt_o1_handle *value) {
  uint64_t id=known(value->pointer,2);assert(element==element_for(id));
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment));
  assert(value->length<=1 && value->capacity==1 && physical_id(value->pointer,stride)==id);
  unsigned previous=enter(9);uint32_t status=__real_zryna_rt_o1_vec_release_storage(element,value);
  assert(status==0);leave(previous,status);return status;
}
