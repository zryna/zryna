#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(ENTRY_PROTOTYPE);
extern void *__real_malloc(size_t);
extern void __real_free(void *);
static struct { void *pointer; int live; } blocks[128];
static size_t used;
void *__wrap_malloc(size_t size) {
  void *pointer=__real_malloc(size);
  assert(pointer && used<128);
  for(size_t i=0;i<used;i++) assert(!blocks[i].live || blocks[i].pointer!=pointer);
  blocks[used].pointer=pointer;blocks[used].live=1;used++;
  assert(printf("{\"acquire\":%zu,\"bytes\":%zu}\n",used,size)>0);
  return pointer;
}
void __wrap_free(void *pointer) {
  if(!pointer) return;
  for(size_t i=0;i<used;i++) {
    if(blocks[i].live && blocks[i].pointer==pointer) {
      blocks[i].live=0;__real_free(pointer);
      assert(printf("{\"release\":%zu}\n",i+1)>0);return;
    }
  }
  assert(0 && "foreign or duplicate native free");
}
static int32_t argument(const char *text) {
  char *end=NULL;errno=0;long long n=strtoll(text,&end,10);
  assert(!errno && end!=text && !*end && n>=INT32_MIN && n<=INT32_MAX);
  return (int32_t)n;
}
int main(int argc,char **argv) {
  assert(argc==ENTRY_ARGUMENT_COUNT+1);
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);
  int32_t result=zryna_v1_e_entry(ENTRY_CALL);
  for(size_t i=0;i<used;i++) assert(!blocks[i].live);
  assert(printf("{\"returned\":%d}\n",result)>0);return 0;
}
