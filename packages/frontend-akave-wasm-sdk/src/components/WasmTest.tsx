"use client";
import init, { AkaveWebSDK } from "akave-wasm-sdk";
import { useAccount } from "wagmi";

const WasmTest = () => {
  const { address } = useAccount();
  const handleOnWasmGreetClick = async () => {
    await init();
    const akaveWasmSdk = await AkaveWebSDK.new();
    const bucketsResult = await akaveWasmSdk.listBuckets(address as string);
    console.log("list_buckets:");
    console.log(bucketsResult.buckets.map((b) => b.name));
    const bucketsInfoResult = bucketsResult.buckets.map((bInfoRes) =>
      akaveWasmSdk.viewBucket(address as string, bInfoRes.name),
    );
    console.log("view_bucket: (in loop)");
    console.log((await Promise.all(bucketsInfoResult)).map((info) => info));

    console.log("list_files: (in loop / bucket)");
    const files = (
      await Promise.all(
        bucketsResult.buckets.map((b) =>
          akaveWasmSdk.listFiles(address as string, b.name),
        ),
      )
    ).map((f) => f.list);
    console.log(
      files
        .map(
          (fb, i) =>
            `${bucketsResult.buckets[i].name}:\n${fb.map((f) => f.rootCid).reduce((prev, curr) => prev + "\n" + curr, "")}`,
        )
        .reduce((prev, curr) => prev + "\n" + curr, ""),
    );
    console.log("view_file_info: (in loop)");
    const fileList = files.map((fs, i) =>
      fs
        .filter((f) => Boolean(f.name))
        .map((f) => {
          console.log({ name: f.name });
          return akaveWasmSdk.viewFileInfo(
            address as string,
            f.name,
            bucketsResult.buckets[i].name,
          );
        }),
    );
    console.log(
      fileList
        .map(async (fb, i) => {
          const nfb = await Promise.all(fb);
          return `${bucketsResult.buckets[i].name}:\n${nfb.map((f) => f).reduce((prev, curr) => prev + "\n" + curr, "")}`;
        })
        .reduce((prev, curr) => prev + "\n" + curr, ""),
    );
  };
  return <button onClick={handleOnWasmGreetClick}>Run greet</button>;
};

export default WasmTest;
