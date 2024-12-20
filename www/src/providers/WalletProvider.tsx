import {
  getDefaultConfig,
  RainbowKitProvider,
  darkTheme,
  lightTheme,
} from "@rainbow-me/rainbowkit";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { FC, PropsWithChildren } from "react";
import { WagmiProvider } from "wagmi";
import "@rainbow-me/rainbowkit/styles.css";
import useTheme from "../hooks/useTheme";
import { fuji } from "../utils/akave";

const queryClient = new QueryClient();

const config = getDefaultConfig({
  appName: "Akave Wasm SDK",
  projectId: import.meta.env.VITE_REOWN_PROJECT_ID,
  chains: [fuji],
  ssr: true, // If your dApp uses server side rendering (SSR)
});

const WalletProvider: FC<PropsWithChildren> = ({ children }) => {
  const [themeName] = useTheme();
  const theme = themeName === "dark" ? darkTheme() : lightTheme();
  return (
    <WagmiProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider theme={theme}>{children}</RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
};

export default WalletProvider;
