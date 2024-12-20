"use client";
import init, { list_buckets } from "akave-wasm-sdk";
import { useAccount } from "wagmi";

const WasmTest = () => {
  const { address } = useAccount();
  const handleOnWasmGreetClick = async () => {
    await init();
    const result = await list_buckets(address as string);
    console.log(result);
  };

  return <button onClick={handleOnWasmGreetClick}>Run greet</button>;
};

export default WasmTest;
