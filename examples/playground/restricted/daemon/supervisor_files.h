/* The driver uploads first, then attaches these two private tmpfs mounts. */
static void mount_flags(const char *path,long flags,long magic,int writable) {
  long data[15]={0};
  require(sc(137,(long)path,(long)data,0)==0,"PLAYGROUND-HOST-MOUNT\n");
  require((data[10]&flags)==flags && (!magic || data[0]==magic)
    && (!writable || !(data[10]&1)),"PLAYGROUND-HOST-MOUNT\n");
}
static long directory(const char *path,unsigned uid,unsigned mode) {
  long fd=sc(257,-100,(long)path,0xb0000);
  require(fd>=0,"PLAYGROUND-HOST-DIRECTORY\n");
  struct file_status status={0};file_stat(fd,&status);
  require(status.uid==uid && status.gid==uid && (status.mode&0177777)==(0040000|mode),
          "PLAYGROUND-HOST-DIRECTORY\n");
  return fd;
}
static void directory_members(long fd,int input) {
  unsigned char buffer[512];unsigned count=0;
  for(;;) {
    long got=sc(217,fd,(long)buffer,sizeof(buffer));
    if(got==-4)continue;
    require(got>=0 && got<=(long)sizeof(buffer),"PLAYGROUND-HOST-INVENTORY\n");
    if(!got)break;
    unsigned long offset=0;
    while(offset<(unsigned long)got) {
      require((unsigned long)got-offset>=20,"PLAYGROUND-HOST-INVENTORY\n");
      unsigned size=(unsigned)buffer[offset+16]|((unsigned)buffer[offset+17]<<8);
      require(size>=20 && size<=(unsigned long)got-offset,"PLAYGROUND-HOST-INVENTORY\n");
      char *name=(char *)(buffer+offset+19);unsigned length=0;
      while(length<size-19 && name[length])++length;
      require(length<size-19,"PLAYGROUND-HOST-INVENTORY\n");
      if(!same(name,".") && !same(name,"..")) {
        require(input && same(name,"request") && ++count==1,"PLAYGROUND-HOST-INVENTORY\n");
      }
      offset+=size;
    }
  }
  require(count==(unsigned)input,"PLAYGROUND-HOST-INVENTORY\n");
}
static long verified_input(unsigned long maximum) {
  mount_flags("/",1,0,0);
  mount_flags("/input",15,0x01021994,0);
  mount_flags("/output",14,0x01021994,1);
  mount_flags("/tmp",14,0x01021994,1);
  long root=directory("/input",0,0555);directory_members(root,1);
  long fd=sc(257,root,(long)"request",0xa0800);
  require(fd>=0,"PLAYGROUND-HOST-INPUT\n");
  struct file_status parent={0},file={0};file_stat(root,&parent);file_stat(fd,&file);
  require(file.dev==parent.dev && file.uid==0 && file.gid==0 && file.nlink==1
    && (file.mode&0177777)==0100444 && file.size>0 && (unsigned long)file.size<=maximum,
    "PLAYGROUND-HOST-INPUT\n");
  close_fd(root);return fd;
}
static long output_file(long root,const char *name) {
  long fd=call6(257,root,(long)name,0xa00c1,0600,0,0);
  require(fd>=0 && sc(91,fd,0600,0)==0,"PLAYGROUND-HOST-OUTPUT\n");
  struct file_status value={0};file_stat(fd,&value);
  require(value.uid==65534 && value.gid==65534 && value.nlink==1 && value.size==0
    && (value.mode&0177777)==0100600,"PLAYGROUND-HOST-OUTPUT\n");
  return fd;
}
static void verify_output(long fd,unsigned long bytes) {
  struct file_status value={0};file_stat(fd,&value);
  require(value.uid==65534 && value.gid==65534 && value.nlink==1
    && value.size>=0 && (unsigned long)value.size==bytes
    && (value.mode&0177777)==0100600,"PLAYGROUND-HOST-OUTPUT\n");
}
