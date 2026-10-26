function $utf8($bytes,$length){
if(!$integer($length)||$length!==$bytes.length)return false;
for(let $i=0;$i<$length;){const $a=$bytes[$i++];if(!$integer($a)||$a>255)return false;if($a<128)continue;
let $n=0,$low=128,$high=191;
if($a>=194&&$a<=223)$n=1;
else if($a>=224&&$a<=239){$n=2;if($a===224)$low=160;if($a===237)$high=159;}
else if($a>=240&&$a<=244){$n=3;if($a===240)$low=144;if($a===244)$high=143;}else return false;
if($i+$n>$length)return false;
const $b=$bytes[$i++];if(!$integer($b)||$b<$low||$b>$high)return false;
for(let $j=1;$j<$n;$j++){const $b=$bytes[$i++];if(!$integer($b)||$b<128||$b>191)return false;}
}return true;
}
function $string($h){if(!$h||$h.brand!==$brand||$h.kind!=='string'||!$integer($h.length)||$h.length!==$h.capacity)return null;
if($h.capacity===0)return $h.pointer===0?$emptyString:null;
const $r=$row($h.pointer);return $r&&$r.size===$h.capacity&&$r.alignment===1?$r:null;}
function $from($bytes,$length){
if(!($bytes instanceof Uint8Array)||!$integer($length)||$bytes.length!==$length)return $error(255);if($length>2147483647)return $error(2);
if(!$utf8($bytes,$length))return $error(4);
const $allocation=$allocate($length,1);if($allocation.status!==0)return $allocation;
const $h=$record({brand:$brand,kind:'string',pointer:$allocation.pointer,length:$length,capacity:$length});
if($length>0){const $r=$row($allocation.pointer);for(let $i=0;$i<$length;$i++)$r.data[$i]=$bytes[$i];}
return $record({status:0,handle:$h});
}
function $clone($source){const $r=$string($source);if(!$r)return $error(255);return $from($r.data,$source.length);}
function $concat($left,$right){
const $a=$string($left),$b=$string($right);if(!$a||!$b)return $error(255);
const $length=$left.length+$right.length;if($length>2147483647)return $error(2);
const $allocation=$allocate($length,1);if($allocation.status!==0)return $allocation;
const $h=$record({brand:$brand,kind:'string',pointer:$allocation.pointer,length:$length,capacity:$length});
if($length>0){const $r=$row($allocation.pointer);for(let $i=0;$i<$left.length;$i++)$r.data[$i]=$a.data[$i];for(let $i=0;$i<$right.length;$i++)$r.data[$left.length+$i]=$b.data[$i];}
return $record({status:0,handle:$h});
}
function $stringFree($h){if(!$string($h))return 255;return $release($h.pointer,$h.capacity,1);}
