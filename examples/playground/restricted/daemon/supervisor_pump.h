/* Bound each pipe independently, including the first extra byte after an exact limit. */
struct stream {long pipe,file;unsigned long bytes,maximum;int open;};
struct poll_descriptor {int fd;short events,revents;};
_Static_assert(sizeof(struct poll_descriptor)==8,"kernel poll descriptor size");
static void terminate_child(long child) {
  long group=sc(62,-child,9,0),direct=sc(62,child,9,0);
  require((group==0 || group==-3) && (direct==0 || direct==-3),"PLAYGROUND-HOST-KILL\n");
  unsigned long deadline=monotonic_ms()+2000;
  for(;;) {
    int status=0;long result=call6(61,child,(long)&status,1,0,0,0);
    if(result==child || result==-10)return;
    require(result==0 || result==-4,"PLAYGROUND-HOST-REAP\n");
    require(monotonic_ms()<deadline,"PLAYGROUND-HOST-REAP\n");pause_tick();
  }
}
__attribute__((noreturn)) static void reject_child(long child,int code) {
  terminate_child(child);stop(code);
}
static int child_exit_status(int status) {
  if(!(status&127))return (status>>8)&255;
  require((status&127)!=127,"PLAYGROUND-HOST-CHILD-STATUS\n");
  return 128+(status&127);
}
static int consume(struct stream *stream,unsigned long deadline) {
  unsigned char bytes[4096];
  unsigned long request=stream->maximum-stream->bytes+1;
  if(request>sizeof(bytes))request=sizeof(bytes);
  long got=sc(0,stream->pipe,(long)bytes,(long)request);
  if(got==-4 || got==-11)return 0;
  if(got<0 || (unsigned long)got>request)return 70;
  if(!got){close_fd(stream->pipe);stream->open=0;return 0;}
  if((unsigned long)got>stream->maximum-stream->bytes)return 74;
  unsigned long offset=0;
  while(offset<(unsigned long)got) {
    if(monotonic_ms()>=deadline)return 124;
    long written=sc(1,stream->file,(long)(bytes+offset),got-(long)offset);
    if(written==-4)continue;
    if(written<=0 || written>got-(long)offset)return 70;
    offset+=(unsigned long)written;
  }
  stream->bytes+=(unsigned long)got;return 0;
}
static int collect(long child,struct stream streams[2],unsigned long deadline) {
  int status=0,reaped=0;
  for(;;) {
    unsigned long now=monotonic_ms();
    if(now>=deadline)reject_child(child,124);
    if(!reaped) {
      long waited=call6(61,child,(long)&status,1,0,0,0);
      if(waited==child)reaped=1;
      else if(waited!=0 && waited!=-4)reject_child(child,70);
    }
    if(reaped && !streams[0].open && !streams[1].open)break;
    struct poll_descriptor pollers[2];
    for(unsigned i=0;i<2;++i) {
      pollers[i].fd=streams[i].open?(int)streams[i].pipe:-1;
      pollers[i].events=1;pollers[i].revents=0;
    }
    long milliseconds=(long)(deadline-now);if(milliseconds>50)milliseconds=50;
    long ready=sc(7,(long)pollers,2,milliseconds);
    if(ready==-4)continue;
    if(ready<0)reject_child(child,70);
    for(unsigned i=0;i<2;++i) {
      if(pollers[i].revents&32)reject_child(child,70);
      if(streams[i].open && (pollers[i].revents&(1|8|16))) {
        int error=consume(&streams[i],deadline);
        if(error)reject_child(child,error);
      }
    }
  }
  for(unsigned i=0;i<2;++i) {
    verify_output(streams[i].file,streams[i].bytes);close_fd(streams[i].file);
  }
  return child_exit_status(status);
}
