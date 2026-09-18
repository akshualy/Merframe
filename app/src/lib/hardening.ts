function isInspectorShortcut(event: KeyboardEvent): boolean {
  if (event.key === "F12") {
    return true;
  }
  const modifier = event.ctrlKey || event.metaKey;
  return (
    modifier &&
    event.shiftKey &&
    ["I", "J", "C"].includes(event.key.toUpperCase())
  );
}

export function hardenProductionWindow() {
  if (!import.meta.env.PROD) {
    return;
  }
  window.addEventListener("contextmenu", (event) => event.preventDefault());
  window.addEventListener("keydown", (event) => {
    if (isInspectorShortcut(event)) {
      event.preventDefault();
    }
  });
}
