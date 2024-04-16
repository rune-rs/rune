/// Hook used to construct an async sleep function.
export function js_sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
