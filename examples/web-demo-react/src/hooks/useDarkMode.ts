import { useEffect, useState } from "react";

const STORAGE_KEY = "theme";
type Theme = "light" | "dark" | "system";

function getSystemTheme(): "light" | "dark" {
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function getStoredTheme(): Theme {
  if (typeof window === "undefined") return "system";
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "dark" || stored === "light") return stored;
  return "system";
}

export function useDarkMode() {
  const [theme, setTheme] = useState<Theme>(() => getStoredTheme());
  const [systemTheme, setSystemTheme] = useState<"light" | "dark">(
    getSystemTheme()
  );

  // Listen for system changes if theme is "system"
  useEffect(() => {
    if (theme !== "system") return;
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      setSystemTheme(e.matches ? "dark" : "light");
      window.document.documentElement.classList.toggle("dark", e.matches);
    };
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [theme]);

  // Apply theme to <html>
  useEffect(() => {
    const root = window.document.documentElement;
    if (theme === "system") {
      const sys = getSystemTheme();
      setSystemTheme(sys);
      root.classList.toggle("dark", sys === "dark");
      localStorage.removeItem(STORAGE_KEY);
    } else {
      root.classList.toggle("dark", theme === "dark");
      localStorage.setItem(STORAGE_KEY, theme);
    }
  }, [theme]);

  // Cycle through themes: light → dark → system → light ...
  const cycleTheme = () => {
    setTheme((prev) =>
      prev === "light" ? "dark" : prev === "dark" ? "system" : "light"
    );
  };

  // The real theme in use
  const resolvedTheme: "light" | "dark" =
    theme === "system" ? systemTheme : theme;

  return { theme, setTheme, cycleTheme, resolvedTheme };
}
