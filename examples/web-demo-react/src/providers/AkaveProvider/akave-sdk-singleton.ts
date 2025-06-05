import init, { AkaveWebSDK, AkaveWebSDKBuilder } from "../../../akave-rs";

const AKAVE_NODE_ADDRESS: string = import.meta.env.VITE_AKAVE_NODE_ADDRESS;

let sdkPromise: Promise<AkaveWebSDK> | null = null;

export function getAkaveSDK(): Promise<AkaveWebSDK> {
  if (!sdkPromise) {
    sdkPromise = (async () => {
      await init();
      const builder = new AkaveWebSDKBuilder(AKAVE_NODE_ADDRESS);
      return await builder.build();
    })();
  }
  return sdkPromise;
}
