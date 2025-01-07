"use client";
import init, { AkaveWebSDK } from "akave-wasm-sdk";
import { useAccount } from "wagmi";

const WasmTest = () => {
  const { address } = useAccount();
  const handleOnWasmGreetClick = async () => {
    await init();
    const akave_wasm_sdk = await AkaveWebSDK.new();
    const result = await akave_wasm_sdk.list_buckets(address as string);
    console.log(result);
  };

  return <button onClick={handleOnWasmGreetClick}>Run greet</button>;
};

export default WasmTest;
