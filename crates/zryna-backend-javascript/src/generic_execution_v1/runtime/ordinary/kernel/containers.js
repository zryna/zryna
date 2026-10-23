function $layout($l){if(!$l||!$integer($l.id))return false;const $expected=$runtimeElements[$l.id];return !!$expected&&$expected.stride===$l.stride&&$expected.alignment===$l.alignment;}
function $validStorage($l,$s){if(!$layout($l))return null;if($s===0)return $emptyStorage;
if(!$s||$s.brand!==$brand||$s.kind!=='vec'||$s.layout!==$l.id||!$integer($s.length)||!$integer($s.capacity)||$s.length>$s.capacity||$s.capacity>1048576)return null;
const $r=$row($s.pointer);return $r&&$r.size===$s.capacity*$l.stride&&$r.alignment===$l.alignment?$s:null;}
function $vecAllocate($l,$capacity){
if(!$layout($l)||!$integer($capacity))return $error(255);
if($capacity>1048576||$capacity*$l.stride>2147483647)return $error(2);
if($capacity===0)return $record({status:0,handle:0});
const $allocation=$allocate($capacity*$l.stride,$l.alignment);if($allocation.status!==0)return $allocation;
return $record({status:0,handle:$record({brand:$brand,kind:'vec',pointer:$allocation.pointer,length:0,capacity:$capacity,layout:$l.id})});
}
function $vecReserve($l,$s,$required){
const $storage=$validStorage($l,$s);if(!$storage||!$integer($required))return $error(255);
if($required>1048576||$required*$l.stride>2147483647)return $error(2);
if($required<=$storage.capacity)return $record({status:0,handle:$s});
let $capacity=$storage.capacity===0?4:$storage.capacity*2;
while($capacity<$required){$capacity*=2;}
if($capacity>1048576||$capacity*$l.stride>2147483647)$capacity=$required;
const $grown=$grow($storage.pointer,$storage.capacity*$l.stride,$capacity*$l.stride,$l.alignment);if($grown.status!==0)return $grown;
return $record({status:0,handle:$record({brand:$brand,kind:'vec',pointer:$grown.pointer,length:$storage.length,capacity:$capacity,layout:$l.id})});
}
function $vecFree($l,$s){const $storage=$validStorage($l,$s);return $storage?$release($storage.pointer,$storage.capacity*$l.stride,$l.alignment):255;}
function $control($c){
if(!$c||Object.getPrototypeOf($c)!==null)return null;const $r=$row($c.$ptr);
if(!$r||$r.size<8||$r.alignment<4||!Number.isInteger($c.$strong)||!Number.isInteger($c.$weak)||$c.$strong<0||$c.$strong>4294967295||$c.$weak<1||$c.$weak>4294967295||typeof $c.$pending!=='boolean')return null;
if($r.control===null){if($c.$strong!==1||$c.$weak!==1||$c.$pending)return null;$r.control=$c;}
if($r.control!==$c)return null;return $r;
}
function $strongClone($c){if(!$control($c)||$c.$pending||$c.$strong===0)return 255;if($c.$strong===4294967295)return 3;$c.$strong++;return 0;}
function $weakDowngrade($c){if(!$control($c)||$c.$pending||$c.$strong===0)return 255;if($c.$weak===4294967295)return 3;$c.$weak++;return 0;}
function $weakClone($c){if(!$control($c)||$c.$pending)return 255;if($c.$weak===4294967295)return 3;$c.$weak++;return 0;}
function $upgrade($c){if(!$control($c)||$c.$pending)return 255;if($c.$strong===0)return 5;if($c.$strong===4294967295)return 3;$c.$strong++;return 0;}
function $strongBegin($c){if(!$control($c)||$c.$pending||$c.$strong===0)return $error(255);$c.$strong--;$c.$pending=$c.$strong===0;return $c.$pending?$true:$false;}
function $strongFinish($c){const $r=$control($c);if(!$r||!$c.$pending||$c.$strong!==0)return 255;$c.$pending=false;$c.$weak--;return $c.$weak===0?$release($c.$ptr,$r.size,$r.alignment):0;}
function $weakFree($c){const $r=$control($c);if(!$r||$c.$pending||$c.$weak<=($c.$strong>0?1:0))return $error(255);$c.$weak--;if($c.$weak===0){if($c.$strong!==0)return $error(255);$release($c.$ptr,$r.size,$r.alignment);return $true;}return $false;}
$runtimeEnter=function(){if($live!==0)$abi();$rows.length=0;$attempts=0;$steps=0;};
$runtimeExit=function(){if($live!==0)$abi();};
