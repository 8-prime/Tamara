import type { z } from "zod";

export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) { super(message); this.status = status; }
}
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Die Anfrage ist fehlgeschlagen. Bitte erneut versuchen.";
}
export function errorStatus(error: unknown): number | undefined {
  return error instanceof ApiError ? error.status : undefined;
}
export function api<T>(schema: z.ZodType<T>, path: string, options?: RequestInit): Promise<T>;
export function api(path: string, options?: RequestInit): Promise<unknown>;
export async function api<T>(schemaOrPath: z.ZodType<T> | string, pathOrOptions?: string | RequestInit, options: RequestInit = {}): Promise<unknown> {
  const schema = typeof schemaOrPath === "string" ? undefined : schemaOrPath;
  const path = typeof schemaOrPath === "string" ? schemaOrPath : String(pathOrOptions);
  const init = typeof schemaOrPath === "string" ? typeof pathOrOptions === "object" ? pathOrOptions : undefined : options;
  const headers = new Headers(init?.headers);
  if (!headers.has("Content-Type") && !(init?.body instanceof FormData)) headers.set("Content-Type", "application/json");
  headers.set("X-Tamara-Request", "1");
  const response = await fetch("/api" + path, { ...init, headers, credentials: "same-origin" });
  if (!response.ok) {
    const data: unknown = await response.json().catch(() => null);
    const message = data && typeof data === "object" && "error" in data && typeof data.error === "string" ? data.error : errorMessage(null);
    throw new ApiError(message, response.status);
  }
  const data: unknown = response.status === 204 ? null : await response.json();
  return schema ? schema.parse(data) : data;
}
