/* Wrap only real source/runtime ABI calls; canonical runtime owns actual allocation/count rules. */
uint32_t __wrap_zryna_rt_o1_allocate(uint64_t size, uint32_t alignment, uintptr_t *out) {
  assert(out && *out==0 && metadata_control(size,alignment)); *out=UINTPTR_MAX;
  if(allocation_failure(0,NULL,0)) return 1;
  uint32_t status=__real_zryna_rt_o1_allocate(size,alignment,out); assert(status==0);
  assert(fprintf(trace,"{\"event\":\"control-metadata\",\"pointer\":\"%" PRIxPTR "\",\"size\":%" PRIu64 ",\"alignment\":%u}\n",*out,size,alignment)>0);
  acquire(*out,size,3,0,0); return status;
}
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes, uint64_t length, zryna_rt_o1_handle *out) {
  poison(out); if(allocation_failure(0,bytes,length)) return 1;
  uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out); assert(status==0 && out->length==length && out->capacity>=length);
  assert(memcmp((const void *)out->pointer,bytes,(size_t)length)==0);
  acquire(out->pointer,out->capacity,1,0,length); return status;
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source, zryna_rt_o1_handle *out) {
  acquired *owner=find(source->pointer,1); assert(source->length<=source->capacity && source->capacity==owner->bytes);
  poison(out); string_clones++;
  record("clone-entry","String",string_clones,source->pointer,source->pointer,(const uint8_t *)source->pointer,source->length);
  if(strcmp(fault,"string-clone")==0 && string_clones==fail_ordinal) {
    record("failure","string-clone",string_clones,0,source->pointer,(const uint8_t *)source->pointer,source->length); return 1;
  }
  if(allocation_failure(source->pointer,(const uint8_t *)source->pointer,source->length)) return 1;
  uint32_t status=__real_zryna_rt_o1_string_clone(source,out); assert(status==0 && out->length==source->length && out->pointer!=source->pointer);
  assert(memcmp((const void *)out->pointer,(const void *)source->pointer,(size_t)source->length)==0);
  acquire(out->pointer,out->capacity,1,source->pointer,out->length); return status;
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  acquired *owner=find(value->pointer,1); assert(owner->bytes==value->capacity);
  if(pending_last) record("payload-drop","String",(size_t)(owner-owners)+1,value->pointer,pending_last,(const uint8_t *)value->pointer,value->length);
  uint32_t status=__real_zryna_rt_o1_string_release(value); assert(status==0 && !owner->live && owner->released==1); return status;
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t element, uint64_t required, zryna_rt_o1_handle *out) {
  uint64_t stride=0; uint32_t alignment=0; assert(metadata_element(element,&stride,&alignment));
  assert(required>0 && required<=2); poison(out);
  if(allocation_failure(0,NULL,0)) return 1;
  uint32_t status=__real_zryna_rt_o1_vec_allocate(element,required,out); assert(status==0 && out->pointer && out->length==0 && out->capacity>=required);
  assert(fprintf(trace,"{\"event\":\"vec-metadata\",\"pointer\":\"%" PRIxPTR "\",\"element\":%u,\"stride\":%" PRIu64 ",\"alignment\":%u,\"required\":%" PRIu64 ",\"capacity\":%" PRIu64 "}\n",out->pointer,element,stride,alignment,required,out->capacity)>0);
  acquire(out->pointer,out->capacity*stride,2,0,0); return status;
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t element, const zryna_rt_o1_handle *value) {
  uint64_t stride=0; uint32_t alignment=0; assert(metadata_element(element,&stride,&alignment));
  acquired *owner=find(value->pointer,2); assert(value->length<=value->capacity && owner->bytes==value->capacity*stride);
  uint32_t status=__real_zryna_rt_o1_vec_release_storage(element,value); assert(status==0 && !owner->live && owner->released==1); return status;
}
uint32_t __wrap_zryna_rt_o1_grow(uintptr_t p,uint64_t a,uint64_t b,uint32_t c,uintptr_t *o) { (void)p;(void)a;(void)b;(void)c;(void)o;abort(); }
uint32_t __wrap_zryna_rt_o1_release(uintptr_t p,uint64_t n,uint32_t a) { (void)p;(void)n;(void)a;abort(); }
uint32_t __wrap_zryna_rt_o1_string_concat(const zryna_rt_o1_handle *a,const zryna_rt_o1_handle *b,zryna_rt_o1_handle *o) { (void)a;(void)b;(void)o;abort(); }
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t element,const zryna_rt_o1_handle *source,uint64_t required,zryna_rt_o1_handle *out) {
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment));
  acquired *owner=find(source->pointer,2); assert(owner->bytes==source->capacity*stride && source->length<required && required==2);
  assert(source->capacity<required && source->length*stride<=64);
  zryna_rt_o1_handle before=*source;uint8_t prefix[64];memcpy(prefix,(const void *)source->pointer,(size_t)(source->length*stride));
  poison(out);reserve_calls++;
  assert(fprintf(trace,"{\"event\":\"reserve-entry\",\"pointer\":\"%" PRIxPTR "\",\"length\":%" PRIu64 ",\"capacity\":%" PRIu64 ",\"required\":%" PRIu64 "}\n",source->pointer,source->length,source->capacity,required)>0);
  if(strcmp(fault,"reserve-capacity-status")==0 && reserve_calls==fail_ordinal) {
    record("failure",fault,reserve_calls,0,source->pointer,NULL,0);assert(memcmp(source,&before,sizeof(before))==0);return 2;
  }
  if(allocation_failure(source->pointer,NULL,0)) {assert(memcmp(source,&before,sizeof(before))==0);return 1;}
  uint32_t status=__real_zryna_rt_o1_vec_reserve(element,source,required,out);
  assert(status==0 && out->pointer!=before.pointer && out->length==before.length && out->capacity>=required);
  assert(!owner->live && owner->released==1);assert(memcmp(prefix,(const void *)out->pointer,(size_t)(before.length*stride))==0);
  assert(fprintf(trace,"{\"event\":\"vec-metadata\",\"pointer\":\"%" PRIxPTR "\",\"element\":%u,\"stride\":%" PRIu64 ",\"alignment\":%u,\"required\":%" PRIu64 ",\"capacity\":%" PRIu64 "}\n",out->pointer,element,stride,alignment,required,out->capacity)>0);
  acquire(out->pointer,out->capacity*stride,2,before.pointer,0);return status;
}
