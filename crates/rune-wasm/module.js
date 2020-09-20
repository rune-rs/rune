/// Hook used to construct an async sleep function.
export function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}