"use client";
import init, { greet } from "akave-wasm-sdk";

const WasmTest = () => {
  const handleOnWasmGreetClick = async () => {
    await init();
    greet("Gil");
  };

  return <button onClick={handleOnWasmGreetClick}>Run greet</button>;
};

export default WasmTest;
