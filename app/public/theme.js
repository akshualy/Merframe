const theme = localStorage.getItem("theme") ?? "system";
const dark =
  theme === "dark" ||
  (theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
document.documentElement.classList.toggle(
  "dark",
  dark || location.hash.startsWith("#/overlay/"),
);
