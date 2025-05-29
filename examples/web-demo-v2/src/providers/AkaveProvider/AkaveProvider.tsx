// src/providers/AkaveProvider.tsx
import React, { useEffect, useState } from "react";
import init, { AkaveWebSDK, AkaveWebSDKBuilder } from "../../../akave-rs";
import { AkaveContext } from "./akave-context.js";

const AKAVE_NODE_ADDRESS: string = import.meta.env.VITE_AKAVE_NODE_ADDRESS;

export const AkaveProvider: React.FC<React.PropsWithChildren> = ({
  children,
}) => {
  const [sdk, setSdk] = useState<AkaveWebSDK | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        await init(); // Initialize WASM
        const builder = new AkaveWebSDKBuilder(AKAVE_NODE_ADDRESS);
        const sdkInstance = await builder.build();
        if (mounted) {
          setSdk(sdkInstance);
          setLoading(false);
        }
      } catch (err) {
        if (mounted) {
          setError(err as Error);
          setLoading(false);
        }
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  return (
    <AkaveContext.Provider value={{ sdk, loading, error }}>
      {children}
    </AkaveContext.Provider>
  );
};
