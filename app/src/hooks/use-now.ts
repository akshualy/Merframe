import { useEffect, useState } from "react";

export function useNow(): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    let timer: number | undefined;
    const handleVisibility = () => {
      window.clearInterval(timer);
      if (document.visibilityState !== "visible") {
        return;
      }
      setNow(Date.now());
      timer = window.setInterval(() => setNow(Date.now()), 1000);
    };
    handleVisibility();
    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, []);

  return now;
}
