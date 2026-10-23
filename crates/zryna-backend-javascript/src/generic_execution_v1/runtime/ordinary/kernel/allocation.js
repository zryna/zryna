const $brand=Object.freeze(Object.create(null));
const $rows=[]; Object.setPrototypeOf($rows,null); let $live=0,$attempts=0,$steps=0;
function $record($fields){return Object.freeze(Object.assign(Object.create(null),$fields));}
const $false=$record({status:0,value:false}),$true=$record({status:0,value:true});
const $errors=Object.freeze([$record({status:0,pointer:0,handle:0,value:false}),$record({status:1,pointer:0,handle:0,value:false}),$record({status:2,pointer:0,handle:0,value:false}),$record({status:3,pointer:0,handle:0,value:false}),$record({status:4,pointer:0,handle:0,value:false}),$record({status:5,pointer:0,handle:0,value:false}),$record({status:255,pointer:0,handle:0,value:false})]);
const $emptyString=$record({data:new Uint8Array(0),size:0}),$emptyStorage=$record({capacity:0,length:0,pointer:0});
function $error($status){return $errors[$status===255?6:$status];}
function $abi(){throw new Error('ZRYNA-RT-ABI');}
function $step(){if(++$steps>$runtimeQuota.transitions)$abi();}
function $integer($v){return Number.isSafeInteger($v)&&$v>=0;}
function $alignment($a){return $integer($a)&&$a>0&&$a<=8&&($a&($a-1))===0;}
function $row($pointer){if(!$pointer||$pointer.brand!==$brand||!$integer($pointer.id))return null;const $r=$rows[$pointer.id];return $r&&$r.live&&$r.token===$pointer?$r:null;}
function $allocate($size,$align){
if(!$integer($size)||!$alignment($align))return $error(255);
if($size===0)return $record({status:0,pointer:0});
if($size>2147483647)return $error(2);
if($attempts>=1048576||$live>=1048576)return $error(2);
if(++$attempts>$runtimeQuota.allocations)$abi();
if($size>67108864)return $error(1);
const $id=$rows.length,$token=$record({id:$id,brand:$brand});
const $result=$record({status:0,pointer:$token});
const $r=Object.assign(Object.create(null),{token:$token,size:$size,alignment:$align,data:null,live:false,control:null});
$rows[$id]=$r;
try{$r.data=new Uint8Array($size);}catch($failure){$rows[$id]=null;if($failure instanceof RangeError)return $error(1);throw $failure;}
$r.live=true;$live++;return $result;
}
function $release($pointer,$size,$align){
if($pointer===0&&$size===0&&$alignment($align))return 0;
const $r=$row($pointer);if(!$r||$r.size!==$size||$r.alignment!==$align)return 255;
if($r.control&&($r.control.$strong!==0||$r.control.$weak!==0||$r.control.$pending))return 255;
$r.live=false;$r.data=null;$live--;return 0;
}
function $grow($pointer,$old,$size,$align){
if(!$integer($old)||!$integer($size)||!$alignment($align))return $error(255);
if($pointer===0){if($old!==0)return $error(255);return $allocate($size,$align);}
const $r=$row($pointer);if(!$r||$r.size!==$old||$r.alignment!==$align)return $error(255);
if($size===$old)return $record({status:0,pointer:$pointer});
if($size===0){$release($pointer,$old,$align);return $record({status:0,pointer:0});}
const $next=$allocate($size,$align);if($next.status!==0)return $next;
const $out=$row($next.pointer);for(let $i=0;$i<Math.min($old,$size);$i++)$out.data[$i]=$r.data[$i];
$release($pointer,$old,$align);return $next;
}
