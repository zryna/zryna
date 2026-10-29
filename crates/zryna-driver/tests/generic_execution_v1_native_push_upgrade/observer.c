/* Native-only fixed-oracle observer linked around the unmodified canonical runtime. */
#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include "observer.h"
#include "observer_host.h"
#include "observer_alloc.h"
#include "observer_counts.h"
static size_t number(const char *text) {
  char *end=NULL;errno=0;unsigned long value=strtoul(text,&end,10);
  assert(errno==0 && end!=text && *end=='\0' && value<=6);return (size_t)value;
}
int main(int argc,char **argv) {
  assert(argc==5);case_name=argv[1];fault=argv[2];fail_ordinal=number(argv[3]);
  assert(strcmp(case_name,"vec-push")==0 || strcmp(case_name,"weak-live")==0 || strcmp(case_name,"weak-expired")==0);
  assert((strcmp(fault,"none")==0)==(fail_ordinal==0));
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);trace=fopen(argv[4],"wx");assert(trace && setvbuf(trace,NULL,_IONBF,0)==0);
  int32_t value=zryna_v1_e_entry();
  assert(strcmp(fault,"none")==0 && !pending_last);
  for(size_t i=0;i<successes;i++) assert(!owners[i].live && owners[i].released==1);
  for(size_t i=0;i<host_successes;i++) assert(!blocks[i].live && blocks[i].owner);
  assert(host_successes==successes);
  assert(fclose(trace)==0);printf("result:%d\n",value);return 0;
}
