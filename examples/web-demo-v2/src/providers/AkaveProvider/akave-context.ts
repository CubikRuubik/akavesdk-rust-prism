import { createContext } from "react";
import { AkaveWebSDK } from "../../../akave-rs";

type AkaveContextType = {
  sdk: AkaveWebSDK | null;
  loading: boolean;
  error: Error | null;
};

export const AkaveContext = createContext<AkaveContextType | undefined>(
  undefined
);
