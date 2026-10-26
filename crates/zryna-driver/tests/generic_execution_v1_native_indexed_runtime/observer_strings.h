uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,uint64_t length,zryna_rt_o1_handle *out) {
  assert(length>0 && length<=64);zero(out);uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out);size_t id=0;
  if(!status) {assert(out->length==length && out->capacity>=length && memcmp((const void *)out->pointer,bytes,(size_t)length)==0);id=acquire(out,1,1,0);}
  assert(fprintf(trace,"{\"event\":\"canonical\",\"op\":\"stringFromUtf8Copy\",\"utf8\":\"")>0);hex(bytes,length);
  assert(fprintf(trace,"\",\"status\":%u",status)>0);if(!status) assert(fprintf(trace,",\"owner\":%zu",id)>0);
  assert(fputs("}\n",trace)>=0);completed(status,out);return status;
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,zryna_rt_o1_handle *out) {
  owner *value=find(source->pointer,1);size_t from=identity(value);assert(source->length==value->length && source->capacity==value->payload_bytes && source->length<=64);
  zryna_rt_o1_handle before=*source;uint8_t bytes[64];memcpy(bytes,(const void *)source->pointer,(size_t)source->length);zero(out);
  uint32_t status=__real_zryna_rt_o1_string_clone(source,out);size_t id=0;
  assert(memcmp(source,&before,sizeof(before))==0 && memcmp(bytes,(const void *)source->pointer,(size_t)source->length)==0);
  if(!status) {assert(out->pointer!=source->pointer && out->length==source->length && memcmp(bytes,(const void *)out->pointer,(size_t)out->length)==0);id=acquire(out,1,1,from);}
  assert(fprintf(trace,"{\"event\":\"canonical\",\"op\":\"stringClone\",\"source\":%zu,\"utf8\":\"",from)>0);hex(bytes,source->length);
  assert(fprintf(trace,"\",\"status\":%u",status)>0);if(!status) assert(fprintf(trace,",\"owner\":%zu",id)>0);
  assert(fputs("}\n",trace)>=0);completed(status,out);return status;
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *source) {
  owner *value=find(source->pointer,1);size_t id=identity(value);assert(source->length==value->length && source->capacity==value->payload_bytes);
  uint32_t status=__real_zryna_rt_o1_string_release(source);assert(status==0 && !value->live && value->released==1);
  assert(fprintf(trace,"{\"event\":\"canonical\",\"op\":\"stringRelease\",\"owner\":%zu,\"status\":%u}\n",id,status)>0);return status;
}
