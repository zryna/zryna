/* Private test entry: actual typed reserved carrier, exact20LE record, no ordinary wrapper. */
#include "observer.h"
#include "allocation.h"
#include "strings.h"
#include "counts.h"
extern uint32_t zryna_gexecution_invoke_v1(int32_t *out_scalar);
static void put32(uint8_t *out,uint32_t value) {for(unsigned i=0;i<4;i++) out[i]=(uint8_t)(value>>(8*i));}
int main(int argc,char **argv) {
  assert(argc==3 && strlen(argv[1])==1 && argv[1][0]>='0' && argv[1][0]<='4');fault_code=(unsigned)(argv[1][0]-'0');
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
  trace_fd=open(argv[2],O_WRONLY|O_CREAT|O_EXCL,0600);assert(trace_fd>=0);
  int32_t scalar=INT32_C(0x13579bdf);uint32_t status=zryna_gexecution_invoke_v1(&scalar);
  assert(!inside_runtime && faults==(fault_code!=0));if(status) assert(scalar==0);
  for(unsigned i=0;i<allocated;i++) assert(!blocks[i].live && blocks[i].owner);
  assert(allocated==owners && close(trace_fd)==0);
  uint8_t bytes[20];put32(bytes,UINT32_C(0x3149475a));put32(bytes+4,status);put32(bytes+8,1);put32(bytes+12,status?0:(uint32_t)scalar);put32(bytes+16,UINT32_C(0x31444e45));
  size_t left=sizeof(bytes);const uint8_t *at=bytes;
  while(left) {ssize_t n=write(STDOUT_FILENO,at,left);if(n<0 && errno==EINTR) continue;assert(n>0);at+=(size_t)n;left-=(size_t)n;}
  return 0;
}
