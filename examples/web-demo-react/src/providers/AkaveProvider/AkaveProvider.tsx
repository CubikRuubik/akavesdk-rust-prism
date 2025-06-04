import React, { useState } from "react";
import { AkaveContext } from "./akave-context.js";
import { getAkaveSDK } from "./akave-sdk-singleton";
import type { AkaveWebSDK } from "../../../akave-rs";
import { useAccount } from "wagmi";

export const AkaveProvider: React.FC<React.PropsWithChildren> = ({
  children,
}) => {
  const [sdk, setSdk] = useState<AkaveWebSDK | null>(null);
  const [loading, setLoading] = useState(true);
  const account = useAccount();

  if (account.isConnected && !sdk) {
    getAkaveSDK().then((sdkInstance) => {
      setSdk(sdkInstance);
      setLoading(false);
    });
  } else if (!account.isConnected && sdk) {
    sdk.free();
    setSdk(null);
    setLoading(true);
  }

  return (
    <AkaveContext.Provider value={{ sdk, loading }}>
      {children}
    </AkaveContext.Provider>
  );
};
