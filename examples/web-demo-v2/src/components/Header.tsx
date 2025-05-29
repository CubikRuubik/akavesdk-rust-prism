import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useDarkMode } from "../hooks/useDarkMode";
import {
  SunIcon,
  MoonIcon,
  ComputerDesktopIcon,
} from "@heroicons/react/24/solid";

function ThemeIcon({ theme }: { theme: "light" | "dark" | "system" }) {
  if (theme === "light") return <SunIcon className="w-5 h-5" />;
  if (theme === "dark") return <MoonIcon className="w-5 h-5" />;
  return <ComputerDesktopIcon className="w-5 h-5" />;
}

function themeLabel(theme: "light" | "dark" | "system") {
  if (theme === "system") return "Auto";
  return theme.charAt(0).toUpperCase() + theme.slice(1);
}

const Header = () => {
  const { theme, cycleTheme } = useDarkMode();
  return (
    <header className="w-full flex items-center justify-between px-6 py-4 shadow bg-white/80 dark:bg-gray-900/80 backdrop-blur sticky top-0 z-10">
      <h1 className="text-2xl font-bold tracking-tight">Web Demo V2</h1>
      <div className="flex items-center gap-4">
        <ConnectButton key={theme} />
        <button
          onClick={() => {
            cycleTheme();
            window.location.reload();
          }}
          className="flex items-center gap-2 px-4 py-2 rounded transition-colors bg-gray-200 dark:bg-gray-800 hover:bg-gray-300 dark:hover:bg-gray-700 font-medium"
          title="Toggle theme"
        >
          <ThemeIcon theme={theme} />
          <span className="sr-only">{themeLabel(theme)}</span>
        </button>
      </div>
    </header>
  );
};

export default Header;
