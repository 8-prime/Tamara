export const workAreas = [
  { id: "parent-conferences", label: "Elterngespräche", icon: "users", global: true },
  { id: "conferences", label: "Konferenzen", icon: "note", global: true },
  { id: "dashboard", label: "Dashboard", icon: "home", global: true },
  { id: "calendar", label: "Kalender", icon: "calendar", global: true },
  { id: "students", label: "Klassen", icon: "users" },
  { id: "notes", label: "Notizen", icon: "note" },
  { id: "performances", label: "Leistungen", icon: "chart" },
  { id: "tasks", label: "Aufgaben", icon: "check" },
  { id: "materials", label: "Materialien", icon: "folder" },
  { id: "settings", label: "Klasseneinstellungen", icon: "settings" },
  { id: "learning", label: "Lernentwicklung", icon: "chart", hidden: true },
  { id: "attendance", label: "Anwesenheit", icon: "users", hidden: true },
  { id: "homework", label: "Hausaufgaben", icon: "check", hidden: true },
  { id: "accounts", label: "Konten verwalten", icon: "users", global: true, hidden: true },
  { id: "usage", label: "Nutzungsstatistik", icon: "chart", global: true, hidden: true },
];
export function readRoute() {
  const match = window.location.pathname.match(
    /^\/classes\/([^/]+)\/([^/]+)\/?$/,
  );
  const globalFeature = window.location.pathname.split("/")[1];
  const query = new URLSearchParams(window.location.search);
  const feature = match?.[2] || globalFeature;
  return {
    classId: match?.[1] || "",
    feature: workAreas.some((a) => a.id === feature) ? feature || "dashboard" : "dashboard",
    tab: query.get("tab") === "history" ? "history" : "open",
    subject: query.get("subject") || "",
    pupil: query.get("pupil") || "",
    lesson: query.get("lesson") || "",
    date: query.get("date") || "",
  };
}
export function workspaceUrl(
  classId: string,
  feature: string,
  tab = "open",
  subject = "",
  lesson = "",
  date = "",
  pupil = "",
) {
  const area = workAreas.find((a) => a.id === feature);
  const path = area?.global
    ? feature === "dashboard"
      ? "/"
      : `/${feature}`
    : classId
      ? `/classes/${encodeURIComponent(classId)}/${feature}`
      : "/";
  const query = new URLSearchParams();
  if (feature === "homework") {
    query.set("tab", tab);
    if (subject) query.set("subject", subject);
  }
  if (feature === "students" && pupil) query.set("pupil", pupil);
  if (lesson) query.set("lesson", lesson);
  if (date) query.set("date", date);
  return path + (query.size ? "?" + query : "");
}
