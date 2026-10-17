/* Fixed trusted UI qualification role; no input/output guest mounts or source selectors. */
static long ui_directory(long parent,const char *name) {
  long fd=sc(257,parent,(long)name,0xb0000);
  require(fd>=0,"PLAYGROUND-UI-DIRECTORY\n");return fd;
}
static void ui_filesystem(const char *path,long magic,long flags,int writable) {
  long values[15]={0};
  require(sc(137,(long)path,(long)values,0)==0 && (!magic || values[0]==magic)
    && (values[10]&flags)==flags && (!writable || !(values[10]&1)),"PLAYGROUND-UI-MOUNT\n");
}
static void ui_kernel_value(long root,const char *name,const char *expected,const char *alternate) {
  long fd=sc(257,root,(long)name,0xa0800);
  require(fd>=0,"PLAYGROUND-UI-CGROUP\n");
  char text[64]={0};long got=sc(0,fd,(long)text,63);
  require(got>0 && got<63 && sc(0,fd,(long)(text+63),1)==0
    && ((got==(long)length(expected) && same(text,expected)) ||
        (alternate && got==(long)length(alternate) && same(text,alternate))),"PLAYGROUND-UI-CGROUP\n");
  close_fd(fd);
}
static void ui_role(void) {
  require(sc(39,0,0,0)==1,"PLAYGROUND-UI-INIT\n");
  ui_filesystem("/",0,1,0);ui_filesystem("/materials",0,1,0);
  ui_filesystem("/tmp",0x01021994,14,1);ui_filesystem("/dev/shm",0x01021994,14,1);
  long root=ui_directory(-100,"/");long materials=ui_directory(root,"materials");
  struct file_status value={0};file_stat(materials,&value);
  require(value.uid==0 && value.gid==0 && value.mode==0040555,"PLAYGROUND-UI-MATERIALS\n");
  close_fd(materials);close_fd(root);
  long sys=ui_directory(-100,"/sys");long fs=ui_directory(sys,"fs");
  long group=ui_directory(fs,"cgroup");
  ui_filesystem("/sys/fs/cgroup",0x63677270,1,0);
  ui_kernel_value(group,"memory.max","536870912\n",0);
  ui_kernel_value(group,"memory.swap.max","0\n",0);
  ui_kernel_value(group,"pids.max","256\n",0);
  ui_kernel_value(group,"pids.current","1\n",0);
  ui_kernel_value(group,"cpu.max","200000 100000\n",0);
  ui_kernel_value(group,"cpuset.cpus.effective","0-1\n","0,1\n");
  ui_kernel_value(group,"cgroup.procs","1\n",0);
  close_fd(group);close_fd(fs);close_fd(sys);
}
