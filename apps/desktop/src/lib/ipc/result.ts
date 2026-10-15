/**
 * The result shape of the generated command functions (`typedError` in `bindings.ts`), for
 * stores that take the commands as an interface so tests can pass fakes (MF F8). The generated
 * file does not export it, so it is declared once here.
 */

import type { CommandError } from "./bindings";

export type Result<T, E = CommandError> = { status: "ok"; data: T } | { status: "error"; error: E };
