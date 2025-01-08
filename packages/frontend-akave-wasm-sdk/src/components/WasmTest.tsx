"use client";
import { useAccount } from "wagmi";
import {
  useAkaveListBuckets,
  useAkaveListFiles,
  useAkaveViewBucket,
} from "../hooks/akave";

const BUCKET_NAME = "DOC_SENDER" as const;

const WasmTest = () => {
  const { address } = useAccount();

  const {
    data: bucketsList,
    isFetched: isBucketsListFetched,
    isError: isBucketsListError,
    error: bucketsListError,
    isLoading: isBucketsListLoading,
  } = useAkaveListBuckets({
    address: address as string,
  });

  const {
    data: bucket,
    isFetched: isViewBucketFetched,
    isError: isViewBucketError,
    error: viewBucketError,
    isLoading: isViewBucketLoading,
  } = useAkaveViewBucket(
    {
      address: address as string,
      bucketName: BUCKET_NAME,
    },
    !isBucketsListLoading,
  );

  const {
    data: files,
    isFetched: isListFilesFetched,
    isError: isListFilesError,
    error: listFilesError,
  } = useAkaveListFiles(
    {
      address: address as string,
      bucketName: BUCKET_NAME,
    },
    !isBucketsListLoading && !isViewBucketLoading,
  );

  return (
    <div className="flex flex-col gap-4">
      {isBucketsListFetched && !isBucketsListError && (
        <div className="flex flex-col gap-4 border-2 border-sky-100 p-4">
          <h2>Buckets:</h2>
          {bucketsList?.buckets.map((buc) => (
            <div key={buc.id}>
              <h3>{buc.name}</h3>
              <p>{buc.createdAt}</p>
              <p>{buc.id}</p>
            </div>
          ))}
        </div>
      )}

      {isViewBucketFetched && !isViewBucketError && (
        <div className="border-2 border-sky-100 p-4">
          <h2>Bucket: {bucket?.name}</h2>
          <p>{bucket?.createdAt}</p>
          <p>{bucket?.id}</p>
        </div>
      )}

      {isListFilesFetched && !isListFilesError && (
        <div className="flex flex-col gap-4 border-2 border-sky-100 p-4">
          <h2>files in {BUCKET_NAME}:</h2>
          {files?.list.map((buc) => (
            <div key={buc.rootCid + buc.name}>
              <h3>{buc.name}</h3>
              <p>{buc.createdAt}</p>
              <p>{buc.rootCid}</p>
              <p>{Math.round(buc.size / 1024)} KB</p>
            </div>
          ))}
        </div>
      )}

      {bucketsListError && (
        <div className="border-2 border-red-400 p-4">
          <h2>{bucketsListError.name}</h2>
          <p>{bucketsListError.message}</p>
        </div>
      )}

      {viewBucketError && (
        <div className="border-2 border-red-400 p-4">
          <h2>{viewBucketError.name}</h2>
          <p>{viewBucketError.message}</p>
        </div>
      )}

      {listFilesError && (
        <div className="border-2 border-red-400 p-4">
          <h2>{listFilesError.name}</h2>
          <p>{listFilesError.message}</p>
        </div>
      )}
    </div>
  );
};

export default WasmTest;
