/* Fixed first-vertical String ABI observer: successful IDs differ from actual pointers. */
#include "ownership-runtime-v1.h"
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
typedef struct { uintptr_t pointer; uint64_t length; int live; unsigned releases; } owner;
static owner owners[2];
static size_t successful, allocation_attempts, clone_calls, expected_allocations, fail_ordinal;
static int fault_kind;
static FILE *trace;
static void event(unsigned tag, size_t id, uintptr_t pointer, uintptr_t source, const uint8_t *bytes, uint64_t length) {
    assert(length < 65536);
    assert(fprintf(trace,"{\"tag\":%u,\"id\":%zu,\"identity\":\"%" PRIxPTR "\",\"sourceIdentity\":\"%" PRIxPTR "\",\"length\":%" PRIu64 ",\"bytes_hex\":\"",tag,id,pointer,source,length)>0);
    for (uint64_t index=0; index<length; index++) assert(fprintf(trace,"%02x",(unsigned)bytes[index])==2);
    assert(fputs("\"}\n",trace)>=0);
}
static size_t find(const zryna_rt_o1_handle *value) {
    assert(value->capacity==value->length);
    for (size_t index=0; index<successful; index++) if (owners[index].pointer==value->pointer && owners[index].live) {
        assert(owners[index].length==value->length && owners[index].releases==0); return index;
    }
    assert(!"duplicate or forged acquired owner"); return 0;
}
static uint32_t fresh(const uint8_t *bytes, uint64_t length, zryna_rt_o1_handle *out, uintptr_t source) {
    allocation_attempts++; assert(allocation_attempts<=expected_allocations);
    if (fault_kind==1 && allocation_attempts==fail_ordinal) { event(4,allocation_attempts,0,source,bytes,length); return 1; }
    assert(successful<2 && length<65536);
    void *pointer=malloc(length ? (size_t)length : 1); assert(pointer);
    for(size_t index=0;index<successful;index++) assert(owners[index].pointer!=(uintptr_t)pointer);
    memcpy(pointer,bytes,(size_t)length);
    owners[successful++]=(owner){(uintptr_t)pointer,length,1,0};
    event(source ? 2u : 1u,successful,(uintptr_t)pointer,source,(const uint8_t *)pointer,length);
    *out=(zryna_rt_o1_handle){(uintptr_t)pointer,length,length}; return 0;
}
uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes, uint64_t length, zryna_rt_o1_handle *out) { return fresh(bytes,length,out,0); }
uint32_t zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source, zryna_rt_o1_handle *out) {
    (void)find(source); clone_calls++;
    if(fault_kind==2 && clone_calls==fail_ordinal) { event(5,clone_calls,0,source->pointer,(const uint8_t *)source->pointer,source->length); return 1; }
    return fresh((const uint8_t *)source->pointer,source->length,out,source->pointer);
}
uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
    size_t index=find(value); event(3,index+1,value->pointer,0,(const uint8_t *)value->pointer,value->length);
    memset((void *)value->pointer,0,(size_t)value->length); free((void *)value->pointer);
    owners[index].live=0; owners[index].releases++; return 0;
}
static size_t number(const char *text) {
    char *end=NULL; errno=0; unsigned long value=strtoul(text,&end,10);
    assert(errno==0 && end!=text && *end=='\0' && value<=2); return (size_t)value;
}
static int32_t result_number(const char *text) {
    char *end=NULL; errno=0; long value=strtol(text,&end,10);
    assert(errno==0 && end!=text && *end=='\0' && value>=INT32_MIN && value<=INT32_MAX); return (int32_t)value;
}
int main(int argc, char **argv) {
    assert(argc==6); const struct rlimit no_core={0,0}; assert(setrlimit(RLIMIT_CORE,&no_core)==0);
    assert(setvbuf(stdout,NULL,_IONBF,0)==0); trace=fopen(argv[5],"wx"); assert(trace); assert(setvbuf(trace,NULL,_IONBF,0)==0);
    fault_kind=strcmp(argv[1],"none")==0 ? 0 : strcmp(argv[1],"allocation")==0 ? 1 : strcmp(argv[1],"clone")==0 ? 2 : -1;
    assert(fault_kind>=0); fail_ordinal=number(argv[2]); expected_allocations=number(argv[4]); assert((fault_kind==0)==(fail_ordinal==0));
    int32_t expected=result_number(argv[3]); int32_t value=zryna_v1_e_entry();
    /* A controlled fault reaching here is a test failure; the Rust owner requires SIGILL. */
    assert(fault_kind==0 && value==expected && allocation_attempts==expected_allocations && successful==expected_allocations);
    assert(clone_calls==(expected_allocations ? 1u : 0u));
    for(size_t index=0;index<successful;index++) assert(!owners[index].live && owners[index].releases==1);
    assert(fclose(trace)==0); printf("result:%d\n",value); return 0;
}
