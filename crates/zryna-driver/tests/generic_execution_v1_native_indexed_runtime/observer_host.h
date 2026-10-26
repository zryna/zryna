/* Actual canonical malloc failures, bases and retirement; no fabricated allocation IDs. */
extern void *__real_malloc(size_t bytes);
extern void __real_free(void *pointer);
typedef struct {uintptr_t base;size_t bytes,owner;unsigned live;} physical;
static physical blocks[5];
static size_t allocated;
void *__wrap_malloc(size_t bytes) {
  assert(bytes>0);attempts++;assert(attempts<=5);
  if(attempts==fail_ordinal) {
    assert(fprintf(trace,"{\"event\":\"attempt\",\"ordinal\":%zu,\"bytes\":%zu,\"failed\":true}\n",attempts,bytes)>0);return NULL;
  }
  void *pointer=__real_malloc(bytes);assert(pointer && allocated<5);
  blocks[allocated++]=(physical){(uintptr_t)pointer,bytes,0,1};
  assert(fprintf(trace,"{\"event\":\"attempt\",\"ordinal\":%zu,\"bytes\":%zu,\"failed\":false,\"base\":\"%" PRIxPTR "\"}\n",attempts,bytes,(uintptr_t)pointer)>0);return pointer;
}
static size_t acquire(const zryna_rt_o1_handle *value,unsigned kind,uint64_t stride,size_t source) {
  assert(value->pointer && successes<5);uint64_t bytes=value->capacity*stride;
  physical *block=NULL;
  for(size_t i=0;i<allocated;i++) if(blocks[i].live && !blocks[i].owner && value->pointer>=blocks[i].base && value->pointer-blocks[i].base<=blocks[i].bytes && bytes<=blocks[i].bytes-(value->pointer-blocks[i].base)) {assert(!block);block=&blocks[i];}
  assert(block);size_t id=++successes;block->owner=id;
  owners[id-1]=(owner){block->base,value->pointer,block->bytes,bytes,value->length,kind,1,0};
  assert(fprintf(trace,"{\"event\":\"acquired\",\"owner\":%zu,\"kind\":\"%s\",\"base\":\"%" PRIxPTR "\",\"pointer\":\"%" PRIxPTR "\",\"allocationBytes\":%zu,\"payloadBytes\":%" PRIu64 ",\"capacity\":%" PRIu64 ",\"source\":%zu,\"utf8\":\"",id,kind==1?"String":"Vec buffer",block->base,value->pointer,block->bytes,bytes,value->capacity,source)>0);
  if(kind==1) hex((const uint8_t *)value->pointer,value->length);
  assert(fputs("\"}\n",trace)>=0);return id;
}
void __wrap_free(void *pointer) {
  assert(pointer);
  for(size_t i=0;i<allocated;i++) if(blocks[i].base==(uintptr_t)pointer && blocks[i].live) {
    assert(blocks[i].owner && blocks[i].owner<=successes);owner *value=&owners[blocks[i].owner-1];
    assert(value->live && !value->released);value->live=0;value->released++;blocks[i].live=0;
    assert(fprintf(trace,"{\"event\":\"released\",\"owner\":%zu,\"base\":\"%" PRIxPTR "\",\"pointer\":\"%" PRIxPTR "\",\"utf8\":\"",identity(value),value->base,value->pointer)>0);
    if(value->kind==1) hex((const uint8_t *)value->pointer,value->length);
    assert(fputs("\"}\n",trace)>=0);__real_free(pointer);return;
  }
  assert(!"free must name one live actual canonical allocation base");
}
