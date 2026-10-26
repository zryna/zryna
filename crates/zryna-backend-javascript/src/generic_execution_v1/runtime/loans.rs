//! Module-private data-only address lens helpers; they create no runtime resource/status.
pub(super) const HELPERS: &str = r"function $address($base,$path) {
Object.setPrototypeOf($path,null); Object.freeze($path);
return Object.freeze(Object.assign(Object.create(null),{base:$base,path:$path}));
}
function $project($parent,$suffix) {
const $path=new Array($parent.path.length+$suffix.length); Object.setPrototypeOf($path,null);
for(let $i=0;$i<$parent.path.length;$i++) $path[$i]=$parent.path[$i];
for(let $i=0;$i<$suffix.length;$i++) $path[$parent.path.length+$i]=$suffix[$i];
return $address($parent.base,$path);
}
function $loanGet($address) {
let $value=$address.base; for(let $i=0;$i<$address.path.length;$i++) $value=$value[$address.path[$i]];
return $value;
}
function $loanSet($address,$value) {
let $base=$address.base; for(let $i=0;$i+1<$address.path.length;$i++) $base=$base[$address.path[$i]];
$base[$address.path[$address.path.length-1]]=$value;
}
";
