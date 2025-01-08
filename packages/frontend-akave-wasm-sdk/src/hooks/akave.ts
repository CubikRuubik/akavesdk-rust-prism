import { useQuery } from "@tanstack/react-query";
import { useContext } from "react";
import { AkaveContext } from "../providers/AkaveProvider/context";

type AkaveListBuckets = {
  address: string;
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
  address: string;
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
  address: string;
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
  address: string;
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
