import { createContext } from "react";
import { type AkaveWebSDK } from "../../../akave-rs";

type AkaveContextType = {
  sdk: AkaveWebSDK | null;
  loading: boolean;
};

export const AkaveContext = createContext<AkaveContextType | undefined>(
  undefined
);
