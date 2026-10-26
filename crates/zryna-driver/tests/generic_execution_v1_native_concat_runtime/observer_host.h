/* The canonical runtime owns alignment/header bookkeeping and actual payload allocation. */
extern void *__real_malloc(size_t size);
extern void __real_free(void *pointer);
void *__wrap_malloc(size_t size) {
  assert(inside_runtime && trace && attempts<6);attempts++;
  if(strcmp(fault,"allocation")==0 && attempts==fail_ordinal) {
    record("failure","allocation",attempts,0,0,0,NULL,0);return NULL;
  }
  assert(host_successes<6);void *pointer=__real_malloc(size);assert(pointer);
  blocks[host_successes++]=(physical){(uintptr_t)pointer,size,0,1};
  assert(fprintf(trace,"{\"event\":\"host-malloc\",\"id\":%zu,\"base\":\"%" PRIxPTR "\",\"bytes\":%zu}\n",host_successes,(uintptr_t)pointer,size)>0);
  return pointer;
}
void __wrap_free(void *pointer) {
  assert(inside_runtime && pointer);
  for(size_t i=0;i<host_successes;i++) if(blocks[i].live && blocks[i].base==(uintptr_t)pointer) {
    assert(blocks[i].owner && blocks[i].owner<=successes);
    acquired *owner=&owners[blocks[i].owner-1];assert(owner->live);
    record("release","String",identity(owner),owner->pointer,owner->base,0,(const uint8_t *)owner->pointer,owner->length);
    owner->live=0;blocks[i].live=0;
    record("host-free","allocation",i+1,0,(uintptr_t)pointer,0,NULL,0);
    __real_free(pointer);return;
  }
  assert(!"free must use one actual live canonical allocation base");
}
