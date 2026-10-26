/* Existing container JSON records: fixed stack buffers, bounded BEFORE direct writes. */
#include "mixed-observer.h"
#include "native_payloads.h"
static int json_fd;
static size_t json_bytes;
typedef struct { uintptr_t pointer;uint64_t bytes;unsigned kind; } published;
static published handles[27];
void json_start(const char *path) { json_fd=open(path,O_WRONLY|O_CREAT|O_EXCL,0600);assert(json_fd>=0); }
void json_finish(void) { assert(close(json_fd)==0); }
static void json_write(const char *data,size_t bytes) {
  if(bytes>65536U-json_bytes)_exit(124);
  json_bytes+=bytes;
  while(bytes) {ssize_t n=write(json_fd,data,bytes);if(n<0 && errno==EINTR)continue;
    assert(n>0);data+=(size_t)n;bytes-=(size_t)n;}
}
static void record(const char *event,const char *kind,uint64_t id,uintptr_t pointer,
    uintptr_t source,const uint8_t *bytes,uint64_t length) {
  char row[1024];assert(length<=7);
  int n=snprintf(row,sizeof(row),"{\"event\":\"%s\",\"kind\":\"%s\",\"id\":%" PRIu64
      ",\"pointer\":\"%" PRIxPTR "\",\"source\":\"%" PRIxPTR
      "\",\"length\":%" PRIu64 ",\"bytes_hex\":\"",event,kind,id,pointer,source,length);
  assert(n>0 && (size_t)n<sizeof(row));size_t at=(size_t)n;
  for(uint64_t i=0;i<length;i++) {assert(at+2<sizeof(row));
    int added=snprintf(row+at,sizeof(row)-at,"%02x",(unsigned)bytes[i]);assert(added==2);at+=2;}
  assert(at+4<sizeof(row));memcpy(row+at,"\"}\n",3);at+=3;json_write(row,at);
}
uint64_t known(uintptr_t pointer,unsigned kind) {
  for(uint64_t id=1;id<=26;id++)if(handles[id].pointer==pointer && physical_live(id)) {
    assert(handles[id].kind==kind);return id;
  }
  assert(!"unknown, stale or wrong-kind canonical pointer");return 0;
}
uint64_t check_string(const zryna_rt_o1_handle *value) {
  assert(value);uint64_t id=known(value->pointer,1);const expected_row *row=&expected_rows[id];
  assert(value->length==row->length && value->capacity==row->length);
  assert(handles[id].bytes==value->capacity && physical_id(value->pointer,value->capacity)==id);
  assert(memcmp((const void *)value->pointer,row->bytes,row->length)==0);return id;
}
void publish_storage(uintptr_t pointer,uint64_t bytes,unsigned kind) {
  uint64_t id=physical_id(pointer,bytes);assert(id<=26 && id==successes);
  assert(expected_rows[id].kind==kind && !handles[id].pointer);
  for(uint64_t i=1;i<id;i++)assert(!physical_live(i) || handles[i].pointer!=pointer);
  handles[id]=(published){pointer,bytes,kind};
}
void publish_string(const zryna_rt_o1_handle *out,uintptr_t source) {
  publish_storage(out->pointer,out->capacity,1);uint64_t id=check_string(out);
  unsigned from=expected_rows[id].source;
  assert(source==(from ? handles[from].pointer : 0));
  if(from)assert(physical_live(from));
  record("acquire","String",id,out->pointer,source,(const uint8_t *)out->pointer,out->length);
}
void release_string(const zryna_rt_o1_handle *value) {
  uint64_t id=check_string(value);
  if(pending_last)assert(expected_rows[known(pending_last,3)].source==id);
  record("release","String",id,value->pointer,0,(const uint8_t *)value->pointer,value->length);
}
void string_failure(unsigned kind,unsigned ordinal,uintptr_t source,const uint8_t *bytes,uint64_t length) {
  record("failure",kind==6 ? "string-clone" : "allocation",ordinal,0,source,bytes,length);
}
void count_record(const char *name,uintptr_t pointer,uint32_t s,uint32_t w,
    uint32_t after_s,uint32_t after_w,uint32_t status) {
  char row[1024];int n=snprintf(row,sizeof(row),"{\"event\":\"count\",\"name\":\"%s\",\"pointer\":\"%" PRIxPTR
    "\",\"beforeStrong\":%u,\"beforeWeak\":%u,\"afterStrong\":%u,\"afterWeak\":%u,\"status\":%u}\n",
    name,pointer,s,w,after_s,after_w,status);assert(n>0 && (size_t)n<sizeof(row));json_write(row,(size_t)n);
}
