import { type Chain } from "viem";

export const fuji = {
  id: 78963,
  name: "Akave Fuji",
  nativeCurrency: { name: "AKVF", symbol: "AKVF", decimals: 18 },
  rpcUrls: {
    default: {
      http: [
        "https://node1-asia.ava.akave.ai/ext/bc/tLqcnkJkZ1DgyLyWmborZK9d7NmMj6YCzCFmf9d9oQEd2fHon/rpc",
      ],
    },
  },
  blockExplorers: {
    default: { name: "Etherscan", url: "https://explorer.akave.network" },
  },
} as const satisfies Chain;
