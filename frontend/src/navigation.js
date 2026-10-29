export const workAreas = [
  { id: "learning", label: "Lernentwicklung", icon: "▦" },
  { id: "attendance", label: "Anwesenheit", icon: "✓" },
  { id: "homework", label: "Hausaufgaben", icon: "▤" },
  { id: "notes", label: "Notizen", icon: "✎" },
  { id: "students", label: "Kinder", icon: "♧" },
  { id: "settings", label: "Klasseneinstellungen", icon: "⚙" },
];

export function readRoute() {
  const match = window.location.pathname.match(
    /^\/classes\/([^/]+)\/([^/]+)\/?$/,
  );
  const query = new URLSearchParams(window.location.search);
  return {
    classId: match?.[1] || "",
    feature: workAreas.some((area) => area.id === match?.[2])
      ? match[2]
      : "learning",
    tab: query.get("tab") === "history" ? "history" : "open",
    subject: query.get("subject") || "",
  };
}

export function workspaceUrl(classId, feature, tab = "open", subject = "") {
  if (!classId) return "/";
  const query = new URLSearchParams();
  if (feature === "homework") {
    query.set("tab", tab);
    if (subject) query.set("subject", subject);
  }
  return `/classes/${encodeURIComponent(classId)}/${feature}${query.size ? "?" + query : ""}`;
}
