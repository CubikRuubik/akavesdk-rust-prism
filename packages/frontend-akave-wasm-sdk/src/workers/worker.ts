import initAkave, { AkaveWebSDK } from "akave-wasm-sdk";

let akaveWebSDK: AkaveWebSDK;

const init = async function () {
  if (akaveWebSDK) return akaveWebSDK;
  await initAkave();
  akaveWebSDK = await AkaveWebSDK.new();
};

onmessage = async function ({ data: { fn, data } }) {
  await init();
  console.log({ fn, data });
  switch (fn) {
    case "uploadFile":
      postMessage(
        await akaveWebSDK.uploadFile(...(data as [string, string, File])),
      );
      return;
  }
  // const workerResult = await runAkave(fn, ...data);
  postMessage(`invalid function: ${fn}`);
};
