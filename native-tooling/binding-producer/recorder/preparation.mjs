// This is intentionally NOT an authenticated final-source selection or an automatic binding producer.
export const selection = null;
export function requireSelection(value) {
  if (value === null) throw new Error('UNBOUND: final-main CI/publication/operator/prebuild selection required.');
  return value;
}
