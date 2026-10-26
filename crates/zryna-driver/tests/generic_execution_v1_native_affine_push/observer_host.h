/* Test-private observation of the canonical runtime's real malloc/free calls. */
extern void *__real_malloc(size_t size);
extern void __real_free(void *pointer);
typedef struct { uintptr_t base; size_t bytes, owner; unsigned live; } physical;
static physical blocks[6];
static size_t host_successes;
void *__wrap_malloc(size_t size) {
  assert(host_successes<6); void *pointer=__real_malloc(size); assert(pointer);
  blocks[host_successes++]=(physical){(uintptr_t)pointer,size,0,1};
  assert(fprintf(trace,"{\"event\":\"host-malloc\",\"id\":%zu,\"base\":\"%" PRIxPTR "\",\"bytes\":%zu}\n",host_successes,(uintptr_t)pointer,size)>0);
  return pointer;
}
static void bind_allocation(uintptr_t pointer, uint64_t bytes, size_t owner) {
  for(size_t i=0;i<host_successes;i++) if(blocks[i].live && !blocks[i].owner && pointer>=blocks[i].base && pointer-blocks[i].base<=blocks[i].bytes && bytes<=blocks[i].bytes-(pointer-blocks[i].base)) {
    assert(i+1==owner);blocks[i].owner=owner;
    assert(fprintf(trace,"{\"event\":\"bind-allocation\",\"id\":%zu,\"base\":\"%" PRIxPTR "\",\"pointer\":\"%" PRIxPTR "\",\"bytes\":%" PRIu64 "}\n",owner,blocks[i].base,pointer,bytes)>0);return;
  }
  assert(!"payload pointer not inside one actual canonical allocation");
}
void __wrap_free(void *pointer) {
  assert(pointer);
  for(size_t i=0;i<host_successes;i++) if(blocks[i].base==(uintptr_t)pointer && blocks[i].live) {
    assert(blocks[i].owner && blocks[i].owner<=successes); acquired *owner=&owners[blocks[i].owner-1];
    assert(owner->live && !owner->released);
    releasing(owner,owner->kind==1 ? (const uint8_t *)owner->pointer : NULL,owner->string_length);
    blocks[i].live=0;
    assert(fprintf(trace,"{\"event\":\"host-free\",\"id\":%zu,\"base\":\"%" PRIxPTR "\"}\n",i+1,(uintptr_t)pointer)>0);
    __real_free(pointer);return;
  }
  assert(!"canonical free requires one live actual allocation base");
}
