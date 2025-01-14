import { useMutation, useQuery } from "@tanstack/react-query";
import { useContext } from "react";
import { AkaveContext } from "../providers/AkaveProvider/context";
import { Address } from "viem";

type TestFn = {
  address: Address;
  bucketName: string;
};

export const useUploadFile = ({ address, bucketName }: TestFn) => {
  const { sdk: akaveSdkCtx } = useContext(AkaveContext);
  return useMutation({
    mutationFn: (filePath: string) => {
      if (!akaveSdkCtx) {
        throw new Error("Akave SDK context not initialized.");
      }
      return akaveSdkCtx.uploadFile(address, bucketName, filePath);
    },
  });
};

type AkaveListBuckets = {
  address: Address;
};

export const useAkaveListBuckets = (
  { address }: AkaveListBuckets,
  enabled = true,
) => {
  const { sdk: akaveSdkCtx } = useContext(AkaveContext);
  return useQuery({
    queryKey: ["akave_list_buckets", address],
    queryFn: () => akaveSdkCtx?.listBuckets(address),
    enabled: !!akaveSdkCtx && !!address && enabled,
  });
};

type AkaveViewBucket = {
  address: Address;
  bucketName: string;
};

export const useAkaveViewBucket = (
  { address, bucketName }: AkaveViewBucket,
  enabled = true,
) => {
  const { sdk: akaveSdkCtx } = useContext(AkaveContext);
  return useQuery({
    queryKey: ["akave_view_bucket", address, bucketName],
    queryFn: () => akaveSdkCtx?.viewBucket(address, bucketName),
    enabled: !!akaveSdkCtx && !!address && enabled,
  });
};

type AkaveViewFileInfo = {
  address: Address;
  bucketName: string;
  fileName: string;
};

export const useAkaveViewFileInfo = (
  { address, bucketName, fileName }: AkaveViewFileInfo,
  enabled = true,
) => {
  const { sdk: akaveSdkCtx } = useContext(AkaveContext);
  return useQuery({
    queryKey: ["akave_view_file_info", address, bucketName, fileName],
    queryFn: () => akaveSdkCtx?.viewFileInfo(address, bucketName, fileName),
    enabled: !!akaveSdkCtx && !!address && enabled,
  });
};

type AkaveListFiles = {
  address: Address;
  bucketName: string;
};

export const useAkaveListFiles = (
  { address, bucketName }: AkaveListFiles,
  enabled = true,
) => {
  const { sdk: akaveSdkCtx } = useContext(AkaveContext);
  return useQuery({
    queryKey: ["akave_list_files", address, bucketName],
    queryFn: () => akaveSdkCtx?.listFiles(address, bucketName),
    enabled: !!akaveSdkCtx && !!address && enabled,
  });
};
