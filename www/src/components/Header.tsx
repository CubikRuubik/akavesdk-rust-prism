import { ConnectButton } from "@rainbow-me/rainbowkit";
import ThemeButton from "./ThemeButton";

const Header = () => {
  return (
    <div className="w-full bg-zinc-200 dark:bg-zinc-800">
      <div className="ml-auto mr-auto flex w-full max-w-[1140px] items-center justify-between p-4">
        <div>Akave WASM SDK</div>
        <div className="flex items-center gap-4">
          <ThemeButton />
          <ConnectButton />
        </div>
      </div>
    </div>
  );
};

export default Header;
