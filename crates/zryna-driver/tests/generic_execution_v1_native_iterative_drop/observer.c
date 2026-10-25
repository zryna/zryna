/* Bounded private observation of an unchanged ordinary object and actual canonical C runtime. */
#define _POSIX_C_SOURCE 200809L
#include "observer.h"
#include <signal.h>
volatile uint8_t *arena;
unsigned active_role,inside_runtime;
size_t live,attempts,successes,releases,string_clones;
int fault_clone;
static int trace_fd;
static uintptr_t owner_seen;
static uint64_t trace_rows;
enum { ROWS=262144, BUCKETS=524288 };
typedef struct { uintptr_t base;size_t bytes;unsigned live; } allocation;
static allocation rows[ROWS];
static uint32_t buckets[BUCKETS];
static uint32_t used(void) { return *(volatile uint32_t *)(arena+0); }
static uint32_t busy(void) { return *(volatile uint32_t *)(arena+4); }
static uintptr_t owner(void) { return *(volatile uintptr_t *)(arena+8); }
void event(uint64_t kind,uint64_t role,uint64_t a,uint64_t b) {
  if(trace_rows>=UINT64_C(1048576)) { _exit(124); }
  trace_rows++;
  const uint64_t row[8]={kind,role,a,b,used(),busy(),owner(),live};
  const uint8_t *data=(const uint8_t *)row;size_t bytes=sizeof(row);
  while(bytes) {ssize_t n=write(trace_fd,data,bytes);if(n<0 && errno==EINTR)continue;
    assert(n>0);data+=(size_t)n;bytes-=(size_t)n;}
}
static size_t bucket(uintptr_t base) {
  uintptr_t key=base>>3;key^=key>>13;key^=key>>7;
  return (size_t)(key&(BUCKETS-1U));
}
static size_t locate(uintptr_t base) {
  size_t slot=bucket(base);
  for(size_t n=0;n<BUCKETS;n++,slot=(slot+1U)&(BUCKETS-1U)) {
    uint32_t id=buckets[slot];if(!id || rows[id-1U].base==base)return slot;
  }
  assert(!"fixed private observation table exhausted");return 0;
}
unsigned enter(unsigned role) {
  unsigned prior=active_role;active_role=role;
  event(1,role,0,0);
  assert(busy()==1 && owner()!=0 && used()<=live);
  if(!owner_seen) { owner_seen=owner(); }
  assert(owner_seen==owner());
  inside_runtime++;return prior;
}
void leave(unsigned previous,uint32_t status) {
  assert(inside_runtime);event(4,active_role,status,0);
  inside_runtime--;active_role=previous;
}
void *__wrap_malloc(size_t bytes) {
  assert(inside_runtime && attempts<ROWS);attempts++;
  void *pointer=__real_malloc(bytes);assert(pointer && successes<ROWS);
  size_t slot=locate((uintptr_t)pointer);
  assert(!buckets[slot] || !rows[buckets[slot]-1U].live);
  rows[successes]=(allocation){(uintptr_t)pointer,bytes,1};
  buckets[slot]=(uint32_t)++successes;live++;
  event(2,active_role,successes,(uintptr_t)pointer);return pointer;
}
void __wrap_free(void *pointer) {
  assert(inside_runtime && pointer);size_t slot=locate((uintptr_t)pointer);
  uint32_t id=buckets[slot];assert(id && rows[id-1U].live && live>0);
  /* A released owner has no frame: all remaining frames witness OTHER live allocations. */
  assert(used()<live);event(3,active_role,id,(uintptr_t)pointer);
  rows[id-1U].live=0;releases++;live--;__real_free(pointer);
}
static uintptr_t address(int64_t delta) {
  uintptr_t base=(uintptr_t)&zryna_v1_e_entry;
  if(delta>=0) {assert(base<=UINTPTR_MAX-(uint64_t)delta);return base+(uint64_t)delta;}
  uint64_t n=(uint64_t)(-(delta+1))+1;assert(base>=n);return base-n;
}
static unsigned number(const char *text,unsigned max) {
  char *end=NULL;errno=0;unsigned long value=strtoul(text,&end,10);
  assert(!errno && end!=text && !*end && value<=max);return (unsigned)value;
}
static void trapped(int signo) {
  event(8,0,(uint64_t)(unsigned)signo,0);
  if(kill(getpid(),signo)!=0) { _exit(125); }
}
int main(int argc,char **argv) {
  assert(argc==5);unsigned depth=number(argv[1],65536),seed=number(argv[2],3);
  fault_clone=(int)number(argv[3],1);
  uint64_t dummy[4]={0,0,0,0};typedef uintptr_t (*get_type)(uint64_t *);
  arena=(volatile uint8_t *)((get_type)address(M416_TLS_DELTA))(dummy);
  assert(arena && ((uintptr_t)arena&15U)==0);
  const struct rlimit core={0,0};assert(setrlimit(RLIMIT_CORE,&core)==0);
  trace_fd=open(argv[4],O_WRONLY|O_CREAT|O_EXCL,0600);assert(trace_fd>=0);
  assert(used()==0 && busy()==0 && owner()==0);
  struct sigaction handler;memset(&handler,0,sizeof(handler));
  handler.sa_handler=trapped;handler.sa_flags=SA_RESETHAND;
  assert(sigemptyset(&handler.sa_mask)==0 && sigaction(SIGILL,&handler,NULL)==0);
  if(seed==1)*(volatile uint32_t *)(arena+0)=1;
  if(seed==2)*(volatile uint32_t *)(arena+4)=1;
  if(seed==3)*(volatile uintptr_t *)(arena+8)=1;
  event(7,0,seed,depth);
  int32_t result=zryna_v1_e_entry((int32_t)depth);
  event(5,0,(uint64_t)(uint32_t)result,releases);
  assert(!seed && !fault_clone && !inside_runtime && live==0 && successes==releases);
  assert(used()==0 && busy()==0 && owner()==0);
  assert(close(trace_fd)==0);printf("result:%d\n",result);return 0;
}
