import { useQuery } from "@tanstack/react-query";
import init, { AkaveWebSDK } from "akave-wasm-sdk";
import { FC, PropsWithChildren } from "react";
import { AkaveContext } from "./context";

const AkaveProvider: FC<PropsWithChildren> = ({ children }) => {
  const { data: sdk, ...data } = useQuery({
    queryKey: ["akave-sdk"],
    queryFn: async () => {
      await init();
      return await AkaveWebSDK.new();
    },
  });
  if (data.error) {
    console.error(data.error);
  }
  return (
    <AkaveContext.Provider value={{ ...data, sdk }}>
      {children}
    </AkaveContext.Provider>
  );
};

export default AkaveProvider;
