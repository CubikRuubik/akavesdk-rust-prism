import { type Chain } from "viem";

export const fuji = {
  id: 78964,
  name: "Akave Fuji",
  nativeCurrency: { name: "AKVF", symbol: "AKVF", decimals: 18 },
  rpcUrls: {
    default: {
      http: [
        "https://n1-us.akave.ai/ext/bc/2JMWNmZbYvWcJRPPy1siaDBZaDGTDAaqXoY5UBKh4YrhNFzEce/rpc",
      ],
    },
  },
  blockExplorers: {
    default: { name: "Etherscan", url: "https://explorer.akave.network" },
  },
} as const satisfies Chain;
