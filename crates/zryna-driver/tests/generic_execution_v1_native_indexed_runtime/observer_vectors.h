static void vector_metadata(uint32_t element,uint64_t stride,uint32_t alignment,uint64_t capacity,size_t id) {
  assert(fprintf(trace,"{\"event\":\"vec-metadata\",\"owner\":%zu,\"element\":%u,\"stride\":%" PRIu64 ",\"alignment\":%u,\"capacity\":%" PRIu64 "}\n",id,element,stride,alignment,capacity)>0);
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t element,uint64_t required,zryna_rt_o1_handle *out) {
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment));assert(required<=2);zero(out);
  uint32_t status=__real_zryna_rt_o1_vec_allocate(element,required,out);size_t id=0;
  if(!status) {assert(!out->length && out->capacity==required);if(required) id=acquire(out,2,stride,0);else assert(!out->pointer);}
  vector_metadata(element,stride,alignment,required,id);
  assert(fprintf(trace,"{\"event\":\"canonical\",\"op\":\"vecAllocate\",\"capacity\":%" PRIu64 ",\"status\":%u",required,status)>0);
  if(!status) {assert(fprintf(trace,",\"owner\":%zu",id)>0);}
  assert(fputs("}\n",trace)>=0);completed(status,out);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t element,const zryna_rt_o1_handle *source,uint64_t required,zryna_rt_o1_handle *out) {
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment));
  owner *value=find(source->pointer,2);size_t old=identity(value);assert(source->capacity==1 && source->length==1 && required==2 && value->payload_bytes==stride && stride<=64);
  zryna_rt_o1_handle before=*source;uint8_t prefix[64];memcpy(prefix,(const void *)source->pointer,(size_t)stride);zero(out);
  uint32_t status=__real_zryna_rt_o1_vec_reserve(element,source,required,out);size_t id=0;
  assert(memcmp(source,&before,sizeof(before))==0);
  if(!status) {assert(out->length==before.length && out->capacity==2 && out->pointer!=before.pointer && !value->live && value->released==1);assert(memcmp(prefix,(const void *)out->pointer,(size_t)stride)==0);id=acquire(out,2,stride,old);}
  else {assert(value->live && !value->released && memcmp(prefix,(const void *)source->pointer,(size_t)stride)==0);}
  vector_metadata(element,stride,alignment,2,id);
  assert(fprintf(trace,"{\"event\":\"canonical\",\"op\":\"vecReserve\",\"old\":%zu,\"required\":%" PRIu64 ",\"capacity\":2,\"status\":%u",old,required,status)>0);
  if(!status) {assert(fprintf(trace,",\"owner\":%zu",id)>0);}
  assert(fputs("}\n",trace)>=0);completed(status,out);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t element,const zryna_rt_o1_handle *source) {
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment));owner *value=NULL;size_t id=0;
  assert(source->length<=source->capacity);if(source->capacity) {value=find(source->pointer,2);id=identity(value);assert(value->payload_bytes==source->capacity*stride);}else assert(!source->pointer && !source->length);
  uint32_t status=__real_zryna_rt_o1_vec_release_storage(element,source);assert(status==0);if(value) assert(!value->live && value->released==1);
  assert(fprintf(trace,"{\"event\":\"canonical\",\"op\":\"vecReleaseStorage\",\"owner\":%zu,\"status\":%u}\n",id,status)>0);return status;
}
