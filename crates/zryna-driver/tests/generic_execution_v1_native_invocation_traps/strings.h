/* Declared failures keep output zero; input bytes and successful canonical handles are exact. */
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,uint64_t length,zryna_rt_o1_handle *out) {
  assert(bytes && length && length<=8);zero(out);from_calls++;enter(3);
  if((fault_code==1 || fault_code==2 || fault_code==4) && from_calls==2) {
    assert(owners==2 && bytes[0]=='!' && length==1);faults++;record(6,3,from_calls,fault_code,0,0,0,0);zero(out);return leave(3,fault_code);
  }
  uint64_t before=payload((uintptr_t)bytes,length);uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out);
  assert(!status && out->length==length && payload(out->pointer,length)==before && payload((uintptr_t)bytes,length)==before);
  acquire(out->pointer,out->capacity,out->length,1,1,0);return leave(3,status);
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *value,zryna_rt_o1_handle *out) {
  block *owner=find(value->pointer,1);assert(value!=out && value->length==owner->length && value->capacity==owner->capacity);zero(out);
  zryna_rt_o1_handle before=*value;uint64_t bytes=payload(value->pointer,value->length);enter(4);
  uint32_t status=__real_zryna_rt_o1_string_clone(value,out);assert(!status && out->pointer!=value->pointer && out->length==value->length && payload(out->pointer,out->length)==bytes);
  assert(owner->live);immutable(value,&before,bytes);acquire(out->pointer,out->capacity,out->length,1,1,owner->owner);return leave(4,status);
}
uint32_t __wrap_zryna_rt_o1_string_concat(const zryna_rt_o1_handle *a,const zryna_rt_o1_handle *b,zryna_rt_o1_handle *out) {
  block *left=find(a->pointer,1);block *right=find(b->pointer,1);assert(a->length==left->length && b->length==right->length && a->capacity==left->capacity && b->capacity==right->capacity && out!=a && out!=b);zero(out);
  zryna_rt_o1_handle before_a=*a,before_b=*b;uint64_t aa=payload(a->pointer,a->length),bb=payload(b->pointer,b->length);enter(5);
  uint32_t status=__real_zryna_rt_o1_string_concat(a,b,out);assert(!status && out->pointer!=a->pointer && out->pointer!=b->pointer && out->length==a->length+b->length && out->length<=8);
  assert(memcmp((const void *)out->pointer,(const void *)a->pointer,(size_t)a->length)==0 && memcmp((const void *)(out->pointer+a->length),(const void *)b->pointer,(size_t)b->length)==0);
  assert(left->live && right->live);immutable(a,&before_a,aa);immutable(b,&before_b,bb);acquire(out->pointer,out->capacity,out->length,1,1,left->owner);return leave(5,status);
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  block *owner=find(value->pointer,1);assert(value->length==owner->length && value->capacity==owner->capacity);enter(6);
  uint32_t status=__real_zryna_rt_o1_string_release(value);assert(!status && !owner->live);return leave(6,status);
}
