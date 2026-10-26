// Eight outer control effects share the existing invocation; payload debt is transported.
function $accountControlState($source,$ctx){
$accountCheck($ctx);
if($ctx.pending||$ctx.H<1||$ctx.L<1||!$source||typeof $source!=='object')$accountAbi();
if(!Number.isInteger($source.$strong)||$source.$strong<0||$source.$strong>4294967295||!Number.isInteger($source.$weak)||$source.$weak<1||$source.$weak>4294967295||typeof $source.$pending!=='boolean')$accountAbi();
}
function $controlNew($layout,$payload,$ctx){
if(!$layout||!Number.isSafeInteger($layout.size)||$layout.size<8||!Number.isSafeInteger($layout.alignment)||$layout.alignment<4||$layout.alignment>8||($layout.alignment&($layout.alignment-1))!==0)$accountAbi();
const $out=Object.assign(Object.create(null),{$ptr:0,$strong:0,$weak:0,$pending:false,$payload:undefined,$dropParent:null,$dropType:-1,$dropIndex:-1});
const $possible=$layout.size<=2147483647;
$accountAdmit($ctx,$possible?1:0,$possible?2:0);
const $result=$controlAlloc($layout.size,$layout.alignment);
if(!$result||!Number.isInteger($result.status))$accountAbi();
if($result.status!==0){$accountResult($ctx,false);$status_controlAlloc($result.status);$accountAbi();}
if(!$possible||!$result.pointer||typeof $result.pointer!=='object')$accountAbi();
$out.$ptr=$result.pointer;$out.$strong=1;$out.$weak=1;$out.$payload=$payload;
$accountResult($ctx,true);return $out;
}
function $accountControlCount($source,$ctx,$role){
$accountControlState($source,$ctx);if($source.$pending)$accountAbi();
const $strong=$source.$strong,$weak=$source.$weak;
let $possible,$operation,$status;
switch($role){
case 1:if($strong===0)$accountAbi();$possible=$strong!==4294967295;$operation=$strongClone;$status=$status_strongClone;break;
case 2:if($strong===0)$accountAbi();$possible=$weak!==4294967295;$operation=$downgrade;$status=$status_downgrade;break;
case 3:$possible=$weak!==4294967295;$operation=$weakClone;$status=$status_weakClone;break;
case 4:$possible=$strong!==0&&$strong!==4294967295;$operation=$upgrade;$status=$status_upgrade;break;
default:$accountAbi();
}
$accountAdmit($ctx,0,$possible?1:0);
const $result=$operation($source);
if(!Number.isInteger($result))$accountAbi();
if($result!==0){$accountResult($ctx,false);$status($result);if($role===4&&$result===5)return 5;$accountAbi();}
if(!$possible)$accountAbi();$accountResult($ctx,true);return $role===4?0:$source;
}
function $countStrong($source,$ctx){return $accountControlCount($source,$ctx,1);}
function $countDowngrade($source,$ctx){return $accountControlCount($source,$ctx,2);}
function $countWeak($source,$ctx){return $accountControlCount($source,$ctx,3);}
function $accountUpgrade($source,$ctx){return $accountControlCount($source,$ctx,4);}
function $accountControlBool($result,$status,$expected){
if(!$result||!Number.isInteger($result.status))$accountAbi();$status($result.status);
if(typeof $result.value!=='boolean'||$result.value!==$expected)$accountAbi();return $result.value;
}
function $accountStrongBegin($source,$ctx){
$accountControlState($source,$ctx);if($source.$pending||$source.$strong===0)$accountAbi();
const $last=$source.$strong===1;$accountRelease($ctx,0);
const $result=$accountControlBool($strongBegin($source),$status_strongBegin,$last);
$accountReleased($ctx);return $result;
}
function $accountStrongFinish($source,$ctx){
$accountControlState($source,$ctx);if(!$source.$pending||$source.$strong!==0)$accountAbi();
// Read after payload cleanup, immediately before canonical Finish can free this control.
const $last=$source.$weak===1;$accountRelease($ctx,$last?1:0);
$status_strongFinish($strongFinish($source));$accountReleased($ctx);
}
function $accountWeakFree($source,$ctx){
$accountControlState($source,$ctx);if($source.$pending||$source.$weak<=($source.$strong>0?1:0))$accountAbi();
const $last=$source.$strong===0&&$source.$weak===1;$accountRelease($ctx,$last?1:0);
$accountControlBool($weakFree($source),$status_weakFree,$last);$accountReleased($ctx);
}
