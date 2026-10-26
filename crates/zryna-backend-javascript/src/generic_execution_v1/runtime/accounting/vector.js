// Vec storage owns one retained release credit; child ownership is transported, never recreated.
function $accountVecCapacity($layout,$storage){
if(!$layout||!Number.isSafeInteger($layout.stride)||$layout.stride<1)$accountAbi();
if($storage===0)return 0;
if(!$storage||!Number.isSafeInteger($storage.capacity)||$storage.capacity<1||$storage.capacity>1048576||$storage.capacity*$layout.stride>2147483647)$accountAbi();
return $storage.capacity;
}
function $accountVecPossible($layout,$required){
return Number.isSafeInteger($required)&&$required>=0&&$required<=1048576&&$required*$layout.stride<=2147483647;
}
function $accountVecResult($ctx,$result,$status,$empty,$retired){
if(!$result||!Number.isInteger($result.status))$accountAbi();
if($result.status!==0){$accountResult($ctx,false);$status($result.status);$accountAbi();}
const $storage=$result.handle;
if($empty?$storage!==0:!$storage||typeof $storage!=='object')$accountAbi();
$accountResult($ctx,true);$ctx.L-=$retired;$accountCheck($ctx);return $storage;
}
function $vecNew($layout,$items,$initialized,$ctx){
Object.setPrototypeOf($items,null);
const $out=Object.assign(Object.create(null),{$storage:0,$items:$items,$length:0,$dropParent:null,$dropType:-1,$dropIndex:-1});
$accountVecCapacity($layout,0);
if(!Number.isSafeInteger($initialized)||$initialized<0||$initialized>$items.length)$accountAbi();
const $possible=$accountVecPossible($layout,$items.length);
$accountAdmit($ctx,$possible&&$items.length>0?1:0,$possible?1:0);
const $storage=$accountVecResult($ctx,$vecAlloc($layout,$items.length),$status_vecAlloc,$items.length===0,0);
$out.$storage=$storage;$out.$length=$initialized;return $out;
}
function $accountVecReserve($layout,$storage,$required,$ctx){
const $capacity=$accountVecCapacity($layout,$storage),$possible=$accountVecPossible($layout,$required);
const $growth=$possible&&$required>$capacity;
$accountCheck($ctx);if($ctx.H<1||($capacity>0&&$ctx.L<1))$accountAbi();
$accountAdmit($ctx,$growth?1:0,0);
return $accountVecResult($ctx,$vecReserve($layout,$storage,$required),$status_vecReserve,$possible&&$required===0&&$capacity===0,$growth&&$capacity>0?1:0);
}
function $vecPush($layout,$vector,$value,$ctx){
const $length=$vector.$length;
const $storage=$accountVecReserve($layout,$vector.$storage,$length+1,$ctx);
$vector.$storage=$storage;$vector.$items[$length]=$value;$vector.$length=$length+1;
}
function $vecStorageFree($layout,$storage,$ctx){
const $capacity=$accountVecCapacity($layout,$storage);$accountRelease($ctx,$capacity>0?1:0);
$status_vecFree($vecFree($layout,$storage));$accountReleased($ctx);
}
