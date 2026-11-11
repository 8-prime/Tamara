export function initials(name: string) {
  return name
    .split(/\s+/)
    .map((part) => part[0])
    .slice(0, 2)
    .join("")
    .toUpperCase();
}

export type AccountRoleFilter = "all" | "admin" | "teacher";
export type AccountSort = "name" | "newest";
interface ListedAccount {
  name: string;
  email: string;
  is_admin: boolean;
  created_at: number;
}

const collator = new Intl.Collator("de", { sensitivity: "base" });

/** Filters by name or email (case-insensitive) and role, then sorts a copy. */
export function filterAccounts<T extends ListedAccount>(
  accounts: T[],
  query: string,
  role: AccountRoleFilter = "all",
  sort: AccountSort = "name",
): T[] {
  const needle = query.trim().toLocaleLowerCase("de");
  return accounts
    .filter(
      (account) =>
        (role === "all" || (role === "admin") === account.is_admin) &&
        (!needle ||
          account.name.toLocaleLowerCase("de").includes(needle) ||
          account.email.toLocaleLowerCase("de").includes(needle)),
    )
    .sort((a, b) =>
      sort === "newest"
        ? b.created_at - a.created_at || collator.compare(a.name, b.name)
        : collator.compare(a.name, b.name) || a.email.localeCompare(b.email),
    );
}
