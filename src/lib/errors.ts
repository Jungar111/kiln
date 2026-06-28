/**
 * Render an unknown thrown value as a human message.
 *
 * Tauri commands that fail reject with their serialised error *object* (e.g.
 * `{ code, message }` from `ExecuteCommandError`), not an `Error` — so the naive
 * `String(err)` renders "[object Object]". This pulls the `message` field out.
 */
export function toMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  if (typeof err === 'object' && err !== null) {
    if ('message' in err && typeof err.message === 'string') return err.message;
    try {
      return JSON.stringify(err);
    } catch {
      return '[unserialisable error]';
    }
  }
  return String(err);
}
