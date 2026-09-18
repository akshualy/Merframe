import { ThemeProvider } from "next-themes";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { HashRouter } from "react-router";
import App from "@/App";
import { OVERLAY_HASH } from "@/components/overlay-window";
import { hardenProductionWindow } from "@/lib/hardening";
import "@/globals.css";

hardenProductionWindow();

const container = document.getElementById("root");
if (container) {
  const routed = (
    <HashRouter>
      <App />
    </HashRouter>
  );
  createRoot(container).render(
    <StrictMode>
      {window.location.hash.startsWith(OVERLAY_HASH) ? (
        routed
      ) : (
        <ThemeProvider
          attribute="class"
          defaultTheme="dark"
          enableSystem
          disableTransitionOnChange
        >
          {routed}
        </ThemeProvider>
      )}
    </StrictMode>,
  );
}
