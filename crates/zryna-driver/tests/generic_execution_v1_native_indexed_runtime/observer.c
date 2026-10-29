/* Each invocation is isolated. All expectations live in the immutable Rust/source oracle. */
#include "observer.h"
#include "observer_host.h"
#include "observer_strings.h"
#include "observer_vectors.h"
#include "observer_unused.h"
#include <sys/prctl.h>
static long number(const char *text,long minimum,long maximum) {
  char *end=NULL;errno=0;long value=strtol(text,&end,10);assert(!errno && end!=text && !*end && value>=minimum && value<=maximum);return value;
}
int main(int argc,char **argv) {
  assert(argc==5);fail_ordinal=(size_t)number(argv[1],0,5);
  int32_t argument0=(int32_t)number(argv[3],INT32_MIN,INT32_MAX),argument1=(int32_t)number(argv[4],INT32_MIN,INT32_MAX);
  assert(!M416_ENTRY_BOOL0 || argument0==0 || argument0==1);assert(!M416_ENTRY_BOOL1 || argument1==0 || argument1==1);
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);assert(setvbuf(stdout,NULL,_IONBF,0)==0);
  /* RLIMIT_CORE alone does not exclude a host's piped crash collector. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  trace=fopen(argv[2],"wx");assert(trace && setvbuf(trace,NULL,_IONBF,0)==0);
#if M416_ENTRY_ARGUMENTS == 2
  int32_t result=zryna_v1_e_entry(argument0,argument1);
#elif M416_ENTRY_ARGUMENTS == 1
  (void)argument1;int32_t result=zryna_v1_e_entry(argument0);
#else
  (void)argument0;(void)argument1;int32_t result=zryna_v1_e_entry();
#endif
  assert(!fail_ordinal);
  for(size_t i=0;i<successes;i++) assert(!owners[i].live && owners[i].released==1);
  for(size_t i=0;i<allocated;i++) assert(!blocks[i].live && blocks[i].owner);
  assert(allocated==successes && fclose(trace)==0);printf("result:%d\n",result);return 0;
}
