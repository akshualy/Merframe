const theme = localStorage.getItem("theme") ?? "system";
const dark =
  theme === "dark" ||
  (theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
const overlay = location.hash.startsWith("#/overlay/");
const root = document.documentElement;

root.classList.toggle("dark", dark || overlay);
if (!overlay) {
  root.style.colorScheme = dark ? "dark" : "light";
}
