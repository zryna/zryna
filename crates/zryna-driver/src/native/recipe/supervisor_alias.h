#ifndef ZRYNA_NATIVE_RECIPE_SUPERVISOR_ALIAS_H
#define ZRYNA_NATIVE_RECIPE_SUPERVISOR_ALIAS_H
static void copy_alias_payloads(void) {
  mount_flags("/input",1,0);
  const char *sources[]={"/input/forbidden.elf","/input/forbidden.so"};
  const char *targets[2][2]={{"/tmp/forbidden.elf","/output/forbidden.elf"},
                           {"/tmp/forbidden.so","/output/forbidden.so"}};
  const long limits[]={65536,131072};unsigned char data[131072];
  for(unsigned i=0;i<2;++i) {
    long input=sc(257,-100,(long)sources[i],0xa0000);
    require(input>=0,"fixed alias payload open failed\n");
    struct file_status original={0};file_stat(input,&original);
    require(original.uid==0 && original.gid==0 && original.nlink==1
      && (original.mode&0177777)==0100555 && original.size>=4096
      && original.size<=limits[i],"fixed alias payload identity/size differs\n");
    long used=0;
    while(used<original.size) {
      long got=sc(0,input,(long)(data+used),original.size-used);
      require(got>0 && got<=original.size-used,"fixed alias payload read failed\n");used+=got;
    }
    char extra=0;require(sc(0,input,(long)&extra,1)==0,"fixed alias payload grew\n");close_fd(input);
    for(unsigned j=0;j<2;++j) {
      long output=call6(257,-100,(long)targets[i][j],0xa00c1,0600,0,0);
      require(output>=0,"private alias payload create failed\n");long written=0;
      while(written<used) {
        long sent=sc(1,output,(long)(data+written),used-written);
        require(sent>0 && sent<=used-written,"private alias payload write failed\n");written+=sent;
      }
      require(sc(91,output,0555,0)==0,"private alias payload chmod failed\n");
      struct file_status status={0};file_stat(output,&status);
      require(status.uid==65534 && status.gid==65534 && status.nlink==1
        && status.size==used && (status.mode&0177777)==0100555,
        "private executable sensitivity identity differs\n");close_fd(output);
    }
  }
}
#endif
