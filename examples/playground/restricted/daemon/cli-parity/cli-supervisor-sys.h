/* Private CLI-only copy of the reviewed Linux x86-64 ABI; finite nofile1024. */
_Static_assert(sizeof(long)==8 && sizeof(void *)==8,"Linux x86-64 LP64 only");
static long call6(long number,long a,long b,long c,long d,long e,long f) {
  register long r10 __asm__("r10")=d;
  register long r8 __asm__("r8")=e;
  register long r9 __asm__("r9")=f;
  long result;
  __asm__ volatile("syscall":"=a"(result):"a"(number),"D"(a),"S"(b),"d"(c),
                   "r"(r10),"r"(r8),"r"(r9):"rcx","r11","memory","cc");
  return result;
}
static long sc(long number,long a,long b,long c) {return call6(number,a,b,c,0,0,0);}
static unsigned long length(const char *text) {
  unsigned long n=0;while(text[n])++n;return n;
}
static int same(const char *left,const char *right) {
  for(;*left && *left==*right;++left,++right) {}
  return *left==*right;
}
__attribute__((noreturn)) static void stop(int code) {
  sc(231,code,0,0);for(;;) {}
}
__attribute__((noreturn)) static void fail(const char *message) {
  sc(1,2,(long)message,(long)length(message));stop(70);
}
static void require(int condition,const char *message) {if(!condition)fail(message);}
static void close_inherited(void) {
  require(sc(436,3,0xffffffffUL,0)==0,"PLAYGROUND-HOST-INHERITED-FDS\n");
}
static unsigned long monotonic_ms(void) {
  unsigned long now[2]={0,0};
  require(sc(228,1,(long)now,0)==0 && now[0]<(~0UL)/1000-20 && now[1]<1000000000,
          "PLAYGROUND-HOST-CLOCK\n");
  return now[0]*1000+now[1]/1000000;
}
static void pause_tick(void) {
  const long tick[2]={0,1000000};
  long result=sc(35,(long)tick,0,0);
  require(result==0 || result==-4,"PLAYGROUND-HOST-SLEEP\n");
}
__attribute__((noreturn)) static void alarm_expired(int number) {
  (void)number;stop(124);
}
__attribute__((naked)) static void signal_restorer(void) {
  __asm__ volatile("mov $15,%rax\nsyscall\nud2");
}
static void kernel_watchdog(void) {
  struct action {void (*handler)(int);unsigned long flags;void (*restorer)(void);unsigned long mask;};
  struct action action={alarm_expired,0x04000000,signal_restorer,0};
  require(call6(13,14,(long)&action,0,8,0,0)==0,"PLAYGROUND-HOST-WATCHDOG\n");
  require(sc(37,12,0,0)==0,"PLAYGROUND-HOST-WATCHDOG\n");
}
static void verify_process_policy(void) {
  for(unsigned fd=0;fd<3;++fd)
    require(sc(72,fd,1,0)>=0,"PLAYGROUND-HOST-STANDARD-FDS\n");
  require(sc(102,0,0,0)==65534 && sc(107,0,0,0)==65534 && sc(104,0,0,0)==65534
    && sc(108,0,0,0)==65534,"PLAYGROUND-HOST-UID\n");
  require(call6(157,39,0,0,0,0,0)==1,"PLAYGROUND-HOST-NNP\n");
  struct {unsigned version;int pid;} header={0x20080522,0};
  unsigned data[6]={0,0,0,0,0,0};
  require(sc(125,(long)&header,(long)data,0)==0,"PLAYGROUND-HOST-CAPABILITIES\n");
  for(unsigned i=0;i<6;++i)require(data[i]==0,"PLAYGROUND-HOST-CAPABILITIES\n");
  for(unsigned i=0;i<64;++i) {
    long value=call6(157,23,i,0,0,0,0);
    require(value==0 || value==-22,"PLAYGROUND-HOST-CAPABILITIES\n");
  }
  require(call6(157,21,0,0,0,0,0)==2,"PLAYGROUND-HOST-SECCOMP\n");
  const unsigned resources[]={4,1,7};
  const unsigned long limits[]={0,268435456,1024};
  for(unsigned i=0;i<3;++i) {
    unsigned long actual[2]={0,0};
    require(sc(97,resources[i],(long)actual,0)==0 && actual[0]==limits[i]
      && actual[1]==limits[i],"PLAYGROUND-HOST-LIMITS\n");
  }
  sc(95,0077,0,0);
}
