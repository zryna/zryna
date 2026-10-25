/* All physical events are the canonical runtime's actual allocation/free bases. */
extern void *__real_malloc(size_t);
extern void __real_free(void *);
void *__wrap_malloc(size_t bytes) {
  assert(inside_runtime==1 && allocated<6);void *pointer=__real_malloc(bytes);assert(pointer);
  blocks[allocated]=(block){(uintptr_t)pointer,0,bytes,0,0,0,1,0};
  record(1,++allocated,(uintptr_t)pointer,bytes,0,0,0,0);return pointer;
}
void __wrap_free(void *pointer) {
  assert(inside_runtime==1 && pointer);
  for(unsigned i=0;i<allocated;i++) if(blocks[i].base==(uintptr_t)pointer && blocks[i].live) {
    block *value=&blocks[i];assert(value->owner && value->pointer);
    record(3,value->owner,value->kind,value->pointer,value->base,0,value->length,value->kind==1?payload(value->pointer,value->length):0);
    value->live=0;__real_free(pointer);return;
  }
  assert(!"free must retire one actual canonical allocation exactly once");
}
uint32_t __wrap_zryna_rt_o1_allocate(uint64_t size,uint32_t alignment,uintptr_t *out) {
  assert(out && !*out && metadata_control(size,alignment));enter(0);
  uint32_t status=__real_zryna_rt_o1_allocate(size,alignment,out);assert(!status && *out);
  acquire(*out,size,0,1,3,0);return leave(0,status);
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t element,uint64_t capacity,zryna_rt_o1_handle *out) {
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment) && capacity==2);zero(out);enter(7);
  uint32_t status=__real_zryna_rt_o1_vec_allocate(element,capacity,out);assert(!status && !out->length && out->capacity==capacity);
  acquire(out->pointer,capacity,0,stride,2,0);return leave(7,status);
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t element,const zryna_rt_o1_handle *value) {
  uint64_t stride=0;uint32_t alignment=0;assert(metadata_element(element,&stride,&alignment) && stride==sizeof(zryna_rt_o1_handle));
  block *owner=find(value->pointer,2);assert(value->length==2 && value->capacity==owner->capacity);
  /* Elements are already dropped: only the still-live storage descriptor may be read. */
  enter(9);uint32_t status=__real_zryna_rt_o1_vec_release_storage(element,value);assert(!status && !owner->live);return leave(9,status);
}
uint32_t __wrap_zryna_rt_o1_grow(uintptr_t p,uint64_t a,uint64_t b,uint32_t c,uintptr_t *o) {(void)p;(void)a;(void)b;(void)c;(void)o;abort();}
uint32_t __wrap_zryna_rt_o1_release(uintptr_t p,uint64_t n,uint32_t a) {(void)p;(void)n;(void)a;abort();}
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t e,const zryna_rt_o1_handle *v,uint64_t n,zryna_rt_o1_handle *o) {(void)e;(void)v;(void)n;(void)o;abort();}
