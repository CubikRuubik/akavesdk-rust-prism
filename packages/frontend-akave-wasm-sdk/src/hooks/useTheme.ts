import { useLocalStorage } from "@uidotdev/usehooks";
import { useLayoutEffect } from "react";

type ThemeType = "light" | "dark";

const useTheme: () => [
  ThemeType,
  () => void,
  React.Dispatch<React.SetStateAction<ThemeType | null>>,
] = () => {
  const [storageTheme, setStorageTheme] = useLocalStorage<ThemeType | null>(
    "theme",
    null,
  );
  const theme =
    storageTheme == null
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : storageTheme;

  const toggleTheme = () => {
    setStorageTheme(theme === "dark" ? "light" : "dark");
  };

  useLayoutEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
  }, [theme]);

  return [theme, toggleTheme, setStorageTheme];
};

export default useTheme;
