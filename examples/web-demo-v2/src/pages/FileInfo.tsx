import { useParams, Link } from "react-router-dom";
import { useDownloadFile, useViewFileInfo } from "../hooks/useAkave";

export default function FileInfoPage() {
  const { bucketName, fileName } = useParams<{
    bucketName: string;
    fileName: string;
  }>();
  const {
    data: fileInfo,
    isLoading,
    error,
  } = useViewFileInfo(
    bucketName || "",
    fileName || "",
    !!bucketName && !!fileName
  );
  const {
    isLoading: downloadLoading,
    error: downloadError,
    refetch: downloadFile,
  } = useDownloadFile(bucketName || "", fileName || "", false);

  const handleDownload = async () => {
    const result = await downloadFile();
    if (result && result.data) {
      const blob = new Blob([result.data]);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = fileName || "file";
      a.click();
      URL.revokeObjectURL(url);
    }
  };

  return (
    <section className="flex flex-col items-center justify-center flex-1 py-24">
      <div className="bg-[rgb(var(--color-secondary)/1)] rounded-xl shadow-lg p-10 flex flex-col items-center min-w-[320px] max-w-lg w-full">
        <h2 className="max-w-full text-lg font-semibold mb-2 text-[rgb(var(--color-text)/1)]">
          File Info:{" "}
          <span
            className="max-w-full block font-mono truncate"
            title={fileName}
          >
            {fileName}
          </span>
        </h2>
        {isLoading && <div>Loading file info...</div>}
        {error && (
          <div className="text-red-500">{(error as Error).message}</div>
        )}
        {fileInfo && (
          <pre
            className="bg-[rgb(var(--color-bg)/1)] rounded p-4 text-sm overflow-x-auto max-w-full"
            style={{ whiteSpace: "pre", wordBreak: "break-all" }}
          >
            {JSON.stringify(fileInfo, null, 2)}
          </pre>
        )}
        <button
          className="
            px-4 py-2 rounded font-semibold transition-colors border
            bg-[rgb(var(--color-secondary)/0.7)] 
            dark:bg-[rgb(var(--color-secondary)/0.7)]
            text-[rgb(var(--color-text)/1)]
            border-[rgb(var(--color-primary)/0.2)]
            hover:bg-[rgb(var(--color-secondary)/1)]
            hover:border-[rgb(var(--color-primary)/0.5)]
            mt-4 cursor-pointer
          "
          onClick={handleDownload}
          disabled={downloadLoading}
        >
          {downloadLoading ? "Downloading..." : "Download"}
        </button>
        {downloadError && (
          <div className="text-red-500 mt-2">
            {(downloadError as Error).message}
          </div>
        )}
        <div className="flex gap-4 mt-6">
          <Link
            to={`/buckets/${encodeURIComponent(bucketName!)}`}
            className="text-sm text-[rgb(var(--color-primary)/1)] hover:underline"
          >
            ← Back to Files
          </Link>
          <Link
            to="/buckets"
            className="text-sm text-[rgb(var(--color-primary)/1)] hover:underline"
          >
            ← Buckets
          </Link>
        </div>
      </div>
    </section>
  );
}
