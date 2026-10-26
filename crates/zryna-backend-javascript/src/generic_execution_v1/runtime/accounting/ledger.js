// Private shared invocation accounting. Canonical17 signatures and their status sets are unchanged.
const $accountBrand=Object.freeze(Object.create(null)); let $accountActive=null;
function $accountAbi(){throw new Error('ZRYNA-RT-ABI');}
function $accountCheck($ctx){
if(!$ctx||$ctx!==$accountActive||$ctx.brand!==$accountBrand)$accountAbi();
if(!Number.isSafeInteger($ctx.A)||!Number.isSafeInteger($ctx.L)||!Number.isSafeInteger($ctx.S)||!Number.isSafeInteger($ctx.H)||$ctx.A<0||$ctx.L<0||$ctx.S<0||$ctx.H<0)$accountAbi();
if($ctx.A>1048576||$ctx.L>$ctx.A||$ctx.L>1048576||$ctx.S>4194304||$ctx.H>4194304||$ctx.S+$ctx.H>4194304)$accountAbi();
}
function $accountStart(){
if($accountActive!==null)$accountAbi();
const $ctx={brand:$accountBrand,A:0,L:0,S:0,H:0,pending:false,positive:0,debt:0};
Object.setPrototypeOf($ctx,null);Object.seal($ctx);
$accountActive=$ctx;return $ctx;
}
function $accountExit($ctx){$accountCheck($ctx);if($ctx.pending||$ctx.L!==0||$ctx.H!==0)$accountAbi();$accountActive=null;}
function $accountAdmit($ctx,$positive,$debt){
$accountCheck($ctx);
if($ctx.pending||($positive!==0&&$positive!==1)||($debt!==0&&$debt!==1&&$debt!==2))$accountAbi();
if($ctx.S+$ctx.H+1+$debt>4194304||$ctx.A+$positive>1048576||$ctx.L+$positive>1048576)throw new Error('zryna.trap.capacity-v1');
$ctx.S++;$ctx.A+=$positive;$ctx.H+=$debt;$ctx.pending=true;$ctx.positive=$positive;$ctx.debt=$debt;
}
function $accountResult($ctx,$ok){
$accountCheck($ctx);if(!$ctx.pending||typeof $ok!=='boolean')$accountAbi();
if($ok)$ctx.L+=$ctx.positive;else $ctx.H-=$ctx.debt;
$ctx.pending=false;$ctx.positive=0;$ctx.debt=0;
}
function $accountRelease($ctx,$positive){
$accountCheck($ctx);
if($ctx.pending||($positive!==0&&$positive!==1)||$ctx.H===0||$ctx.L<$positive)$accountAbi();
$ctx.S++;$ctx.H--;$ctx.pending=true;$ctx.positive=$positive;$ctx.debt=0;
}
function $accountReleased($ctx){
$accountCheck($ctx);if(!$ctx.pending||$ctx.debt!==0)$accountAbi();
$ctx.L-=$ctx.positive;$ctx.pending=false;$ctx.positive=0;
}
function $accountHandle($ctx,$result,$status){
if(!$result||!Number.isInteger($result.status))$accountAbi();
if($result.status!==0){$accountResult($ctx,false);$status($result.status);$accountAbi();}
if($ctx.debt!==1)$accountAbi();const $value=$handle($result,$status);$accountResult($ctx,true);return $value;
}
function $accountLength($source){
if(!$source||!Number.isSafeInteger($source.length)||$source.length<0||$source.length>2147483647)$accountAbi();return $source.length;
}
function $fresh($bytes,$ctx){
const $buffer=new Uint8Array($bytes.length);for(let $i=0;$i<$bytes.length;$i++)$buffer[$i]=$bytes[$i];
const $possible=$bytes.length<=2147483647;
$accountAdmit($ctx,$possible&&$bytes.length>0?1:0,$possible?1:0);
return $accountHandle($ctx,$from($buffer,$bytes.length),$status_from);
}
function $stringClone($source,$ctx){
const $length=$accountLength($source);$accountAdmit($ctx,$length>0?1:0,1);
return $accountHandle($ctx,$clone($source),$status_clone);
}
function $stringConcat($left,$right,$ctx){
const $length=$accountLength($left)+$accountLength($right),$possible=$length<=2147483647;
$accountAdmit($ctx,$possible&&$length>0?1:0,$possible?1:0);
return $accountHandle($ctx,$concat($left,$right),$status_concat);
}
function $free($source,$ctx){
const $length=$accountLength($source);$accountRelease($ctx,$length>0?1:0);
$status_release($release($source));$accountReleased($ctx);
}
