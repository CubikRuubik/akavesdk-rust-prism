"use client";
import { useEffect } from "react";
import init, { greet } from "akave-wasm-sdk";

const WasmTest = () => {
  useEffect(() => {
    init().then(() => {
      greet();
    });
  }, []);
  return null;
};

export default WasmTest;
