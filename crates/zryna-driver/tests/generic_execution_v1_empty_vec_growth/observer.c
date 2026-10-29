/* Actual canonical malloc/free and storage outputs; no replacement allocator/count model. */
#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include "observer.h"
void *__wrap_malloc(size_t size) {
  ++attempts;event("attempt",attempts);
  if(attempts==fail_ordinal) { fault_fired=1;event("failure",attempts);return NULL; }
  assert(!acquired && !block_base);
  void *base=__real_malloc(size);assert(base);block_base=(uintptr_t)base;block_bytes=size;
  event("host-malloc",1);return base;
}
void __wrap_free(void *pointer) {
  assert(releasing && acquired==1 && !released && (uintptr_t)pointer==block_base);
  released=1;event("release",1);__real_free(pointer);
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t id,uint64_t required,zryna_rt_o1_handle *out) {
  layout(id);zero_out(out);assert(required==0);event("vec-allocate",0);
  uint32_t status=__real_zryna_rt_o1_vec_allocate(id,required,out);
  assert(status==0 && !out->pointer && !out->length && !out->capacity && !attempts);
  return status;
}
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t id,const zryna_rt_o1_handle *source,uint64_t required,zryna_rt_o1_handle *out) {
  layout(id);zero_out(out);assert(source!=out && reserves<2);
  ++reserves;event("reserve-entry",reserves);
  zryna_rt_o1_handle before=*source;assert(required>=before.length+1);
  int32_t prefix=0;
  if(reserves==1) assert(!before.pointer && !before.length && !before.capacity);
  else { assert(before.pointer==payload && before.length==1 && before.capacity==4);memcpy(&prefix,(const void *)payload,4);assert(prefix==7); }
  uint32_t status=__real_zryna_rt_o1_vec_reserve(id,source,required,out);
  assert(memcmp(source,&before,sizeof(before))==0);
  if(status!=0) {
    assert(status==1 && fault_fired && attempts==1 && !acquired && !released);
    assert(!out->pointer && !out->length && !out->capacity);
    *out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX};
    event("failed-output-poisoned",1);return status;
  }
  /* ABI section6 fixes4; capacity is a private observer assertion, not source syntax. */
  assert(fprintf(trace,"{\"event\":\"reserve-result\",\"id\":%zu,\"capacity\":%" PRIu64 ",\"length\":%" PRIu64 "}\n",reserves,out->capacity,out->length)>0);
  assert(out->capacity==4 && out->length==before.length && out->pointer);
  if(reserves==1) {
    assert(attempts==1 && !acquired && block_base);
    payload=out->pointer;assert(payload>=block_base && payload-block_base<=block_bytes && 16<=block_bytes-(payload-block_base));
    assert(payload%4==0);acquired=1;event("acquire",1);
    assert(fputs("{\"event\":\"payload-bytes\",\"id\":1,\"bytes\":16}\n",trace)>=0);
  } else {
    assert(out->pointer==before.pointer && attempts==1 && acquired==1);
    int32_t retained=0;memcpy(&retained,(const void *)payload,4);assert(retained==prefix);
  }
  return status;
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t id,const zryna_rt_o1_handle *value) {
  layout(id);event("vec-release",acquired);
  if(fault_fired) assert(!value->pointer && !value->length && !value->capacity && !acquired);
  else {
    assert(value->pointer==payload && value->length==2 && value->capacity==4);
    int32_t elements[2];memcpy(elements,(const void *)payload,sizeof(elements));assert(elements[0]==7 && elements[1]==11);
  }
  releasing=1;uint32_t status=__real_zryna_rt_o1_vec_release_storage(id,value);releasing=0;
  assert(status==0 && released==acquired);return status;
}
int main(int argc,char **argv) {
  assert(argc==3);char *end=NULL;errno=0;unsigned long ordinal=strtoul(argv[1],&end,10);
  assert(!errno && end!=argv[1] && !*end && ordinal<=2);fail_ordinal=(size_t)ordinal;
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);trace=fopen(argv[2],"wx");assert(trace && setvbuf(trace,NULL,_IONBF,0)==0);
  int32_t value=zryna_v1_e_entry();assert(value==18 && !fault_fired && attempts==1 && acquired==1 && released==1 && reserves==2);
  assert(fclose(trace)==0);printf("result:%d\n",value);return 0;
}
