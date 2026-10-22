// This fixed CDP function creates one fixed worker. Source and component bytes stay inert carriers.
export async function evaluatePage() {
  const { input, expected } = await fetch('/payload').then(response => {
    if (!response.ok) throw new Error('PLAYGROUND-EVALUATION-PAYLOAD');
    return response.json();
  });
  const template = await fetch('/template').then(response => {
    if (!response.ok) throw new Error('PLAYGROUND-EVALUATION-TEMPLATE');
    return response.arrayBuffer();
  });
  const bytes = Uint8Array.from(atob(input.frameBase64), character => character.charCodeAt(0));
  const selection = input.selection;
  const worker = new Worker('/worker.mjs', { type: 'module' });
  try {
    return await new Promise((resolve, reject) => {
      worker.onerror = () => reject(new Error('PLAYGROUND-WORKER'));
      worker.onmessage = ({ data }) => {
        if (!data || data.ok !== true || data.error !== null ||
            data.revision !== selection.revision || data.logical !== selection.logical ||
            data.componentSha256 !== selection.componentSha256 ||
            !Number.isInteger(data.value) || Object.is(data.value, -0) ||
            data.value < -2147483648 || data.value > 2147483647 ||
            JSON.stringify(Object.keys(data).sort()) !==
              '["componentSha256","error","logical","ok","revision","value"]') {
          reject(new Error('PLAYGROUND-WORKER-RESPONSE')); return;
        }
        resolve({ revision: data.revision, logical: data.logical,
          componentSha256: data.componentSha256, value: data.value });
      };
      worker.postMessage({ revision: selection.revision, logical: selection.logical,
        args: selection.args, bytes, expected, receipt: input.receipt,
        authority: input.authority, templateBytes: new Uint8Array(template) });
    });
  } finally { worker.terminate(); }
}
