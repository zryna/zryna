// Test-only exports/state in an explicitly unauditable text copy; no product entry flags.
let $testCode=0,$testClonePreflights=0,$testAcquired=[],$testReleased=[];
let $testKernelExit=null,$testAccountExit=null;
export function testOnlyArm(code){
  if(!Number.isInteger(code)||code<0||code>5||$busy||$accountActive!==null)throw new Error('test precondition');
  $testCode=code;$testClonePreflights=0;$testAcquired=[];$testReleased=[];
  $testKernelExit=null;$testAccountExit=null;
}
export function testOnlySnapshot(){
  return {acquired:$testAcquired,released:$testReleased,clone_preflights:$testClonePreflights,
    kernel_exit:$testKernelExit,account_exit:$testAccountExit,busy:$busy,
    account_inactive:$accountActive===null,armed:$testCode};
}
