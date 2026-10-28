export async function api(path, options = {}) {
  const r = await fetch("/api" + path, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      "X-Tamara-Request": "1",
      ...options.headers,
    },
    credentials: "same-origin",
  });
  if (!r.ok) {
    const data = await r.json().catch(() => ({}));
    const e = new Error(
      data.error || "Die Anfrage ist fehlgeschlagen. Bitte erneut versuchen.",
    );
    e.status = r.status;
    throw e;
  }
  return r.status === 204 ? null : r.json();
}
