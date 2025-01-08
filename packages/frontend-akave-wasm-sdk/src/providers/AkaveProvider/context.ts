import { UseQueryResult } from "@tanstack/react-query";
import { AkaveWebSDK } from "akave-wasm-sdk";
import { createContext } from "react";

type contextTypes =
  | "error"
  | "isError"
  | "isPending"
  | "isLoading"
  | "isLoadingError"
  | "isRefetchError"
  | "isSuccess"
  | "isFetched"
  | "isFetching"
  | "isInitialLoading"
  | "isPaused"
  | "isRefetching"
  | "isStale";

export const AkaveContext = createContext<
  Pick<UseQueryResult<AkaveWebSDK, Error>, contextTypes> & {
    sdk: AkaveWebSDK | undefined;
  }
>({
  sdk: undefined,
  error: null,
  isError: false,
  isPending: false,
  isLoading: false,
  isLoadingError: false,
  isRefetchError: false,
  isSuccess: false,
  isFetched: false,
  isFetching: false,
  isInitialLoading: false,
  isPaused: false,
  isRefetching: false,
  isStale: false,
});
