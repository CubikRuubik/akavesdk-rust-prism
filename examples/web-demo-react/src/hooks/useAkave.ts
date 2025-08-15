import { useContext } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { AkaveContext } from "../providers/AkaveProvider/akave-context";
import { useAccount } from "wagmi";

function useAkave() {
  const ctx = useContext(AkaveContext);
  const { address } = useAccount();
  if (!ctx) throw new Error("useAkave must be used within AkaveProvider");
  return { ctx, sdk: ctx.sdk, address: address || "" };
}

// 1. Create Bucket (mutation)
export function useCreateBucket() {
  const { sdk } = useAkave();
  return useMutation({
    mutationFn: (bucket_name: string) => sdk!.createBucket(bucket_name),
  });
}

// 2. Delete Bucket (mutation)
export function useDeleteBucket() {
  const { sdk } = useAkave();
  return useMutation({
    mutationFn: ({ bucket_name }: { bucket_name: string }) =>
      sdk!.deleteBucket(bucket_name),
  });
}

// 3. Delete File (mutation)
export function useDeleteFile() {
  const { sdk } = useAkave();
  return useMutation({
    mutationFn: ({
      bucket_name,
      file_name,
    }: {
      bucket_name: string;
      file_name: string;
    }) => sdk!.deleteFile(bucket_name, file_name),
  });
}

// 4. Download File (query)
export function useDownloadFile(
  bucket_name: string,
  file_name: string,
  enabled = true,
) {
  const { sdk, address } = useAkave();
  return useQuery({
    queryKey: ["downloadFile", address, bucket_name, file_name],
    queryFn: () => sdk!.downloadFile(bucket_name, file_name),
    enabled: !!sdk && enabled,
  });
}

// 5. List Buckets (query)
export function useListBuckets(enabled = true) {
  const { sdk, address } = useAkave();
  return useQuery({
    queryKey: ["listBuckets", address],
    queryFn: () => sdk!.listBuckets(),
    enabled: !!sdk && enabled,
  });
}

// 6. List Files (query)
export function useListFiles(bucket_name: string, enabled = true) {
  const { sdk, address } = useAkave();
  return useQuery({
    queryKey: ["listFiles", address, bucket_name],
    queryFn: () => sdk!.listFiles(bucket_name),
    enabled: !!sdk && enabled,
  });
}

// 7. Upload File (mutation)
export function useUploadFile() {
  const { sdk } = useAkave();
  return useMutation({
    mutationFn: ({
      bucket_name,
      file_name,
      file_content,
    }: {
      bucket_name: string;
      file_name: string;
      file_content: Uint8Array;
    }) => sdk!.uploadFile(bucket_name, file_name, file_content),
  });
}

// 8. View Bucket (query)
export function useViewBucket(bucket_name: string, enabled = true) {
  const { sdk, address } = useAkave();
  return useQuery({
    queryKey: ["viewBucket", address, bucket_name],
    queryFn: () => sdk!.viewBucket(bucket_name),
    enabled: !!sdk && enabled,
  });
}

// 9. View File Info (query)
export function useViewFileInfo(
  bucket_name: string,
  file_name: string,
  enabled = true,
) {
  const { sdk, address } = useAkave();
  return useQuery({
    queryKey: ["viewFileInfo", address, bucket_name, file_name],
    queryFn: () => sdk!.viewFileInfo(bucket_name, file_name),
    enabled: !!sdk && enabled,
  });
}
