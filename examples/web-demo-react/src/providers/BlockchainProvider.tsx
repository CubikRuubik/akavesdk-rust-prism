import "@rainbow-me/rainbowkit/styles.css";

import {
  connectorsForWallets,
  darkTheme,
  lightTheme,
  RainbowKitProvider,
} from "@rainbow-me/rainbowkit";
import { createConfig, http, WagmiProvider } from "wagmi";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { type FC, type PropsWithChildren } from "react";
import { fuji } from "../utils/akave-network";
import { useDarkMode } from "../hooks/useDarkMode";
import { metaMaskWallet } from "@rainbow-me/rainbowkit/wallets";

const PROJECT_ID: string = import.meta.env.VITE_WALLETCONNECT_ID;

const queryClient = new QueryClient();

const connectors = connectorsForWallets(
  [
    {
      groupName: "Recommended",
      wallets: [metaMaskWallet],
    },
  ],
  {
    appName: "Akave-rs WASM",
    projectId: PROJECT_ID,
  },
);

const config = createConfig({
  connectors,
  chains: [fuji],
  transports: {
    [fuji.id]: http(fuji.rpcUrls.default.http[0]),
  },
  ssr: true, // if using SSR
});

const BlockchainProvider: FC<PropsWithChildren> = ({ children }) => {
  const { resolvedTheme } = useDarkMode();
  const rainbowTheme = resolvedTheme === "dark" ? darkTheme() : lightTheme();

  return (
    <WagmiProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider theme={rainbowTheme}>{children}</RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
};

export default BlockchainProvider;
