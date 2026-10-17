/* Fixed readonly UI proc audit; no caller paths, file reads/writes or process creation. */
static void ui_proc_readonly(long root,const char *name,unsigned type) {
  long fd=sc(257,root,(long)name,0x2a0000); /* O_PATH|O_NOFOLLOW|O_CLOEXEC */
  require(fd>=0,"PLAYGROUND-UI-PROC-OPEN\n");
  struct file_status value={0};file_stat(fd,&value);
  require((value.mode&0170000)==type,"PLAYGROUND-UI-PROC-TYPE\n");
  long values[15]={0};
  require(sc(138,fd,(long)values,0)==0 && values[0]==0x9fa0 && (values[10]&15)==15,
          "PLAYGROUND-UI-PROC-READONLY\n");
  close_fd(fd);
}
static void ui_proc_audit(void) {
  long root=ui_directory(-100,"/proc");long values[15]={0};
  require(sc(138,root,(long)values,0)==0 && values[0]==0x9fa0 && (values[10]&14)==14
    && !(values[10]&1),"PLAYGROUND-UI-PROC-ROOT\n");
  ui_proc_readonly(root,"bus",0040000);ui_proc_readonly(root,"fs",0040000);
  ui_proc_readonly(root,"irq",0040000);ui_proc_readonly(root,"sys",0040000);
  long trigger=sc(257,root,(long)"sysrq-trigger",0x2a0000);
  const char *record=0;
  if(trigger==-2) {
    record="{\"format\":\"zryna.playground-ui-proc-audit.v1\",\"version\":1,\"sysrqTrigger\":\"absent-enoent\"}\n";
  } else {
    require(trigger>=0,"PLAYGROUND-UI-PROC-SYSRQ\n");
    struct file_status value={0};file_stat(trigger,&value);
    require((value.mode&0170000)==0100000,"PLAYGROUND-UI-PROC-TYPE\n");
    long observed[15]={0};
    require(sc(138,trigger,(long)observed,0)==0 && observed[0]==0x9fa0 && (observed[10]&15)==15,
            "PLAYGROUND-UI-PROC-READONLY\n");
    close_fd(trigger);
    record="{\"format\":\"zryna.playground-ui-proc-audit.v1\",\"version\":1,\"sysrqTrigger\":\"present-readonly\"}\n";
  }
  close_fd(root);
  require(sc(1,1,(long)record,(long)length(record))==(long)length(record),"PLAYGROUND-UI-PROC-OUTPUT\n");
  stop(0);
}
