import { useParams, Link } from "react-router-dom";
import { useListFiles } from "../hooks/useAkave";
import { FileUploadButton } from "../components/FileUploadButton";

export default function BucketFilesPage() {
  const { bucketName } = useParams<{ bucketName: string }>();
  const {
    data: files,
    isLoading,
    error,
    refetch,
  } = useListFiles(bucketName || "", !!bucketName);

  return (
    <section className="flex flex-col items-center justify-center flex-1 py-24">
      <div className="bg-[rgb(var(--color-secondary)/1)] rounded-xl shadow-lg p-10 flex flex-col items-center min-w-[320px] max-w-lg w-full">
        <h2 className="max-w-full w-full text-lg font-semibold mb-2 text-[rgb(var(--color-text)/1)]">
          File in:{" "}
          <span
            className="max-w-full block font-mono truncate"
            title={bucketName}
          >
            {bucketName}
          </span>
        </h2>
        {bucketName && (
          <FileUploadButton bucketName={bucketName} onSuccess={refetch} />
        )}
        {isLoading && <div>Loading files...</div>}
        {error && (
          <div className="text-red-500">{(error as Error).message}</div>
        )}
        <ul className="space-y-2 w-full">
          {files?.files.map((file) => (
            <li key={`${file.name}-${file.createdAt}`}>
              <Link
                to={`/buckets/${encodeURIComponent(
                  bucketName!,
                )}/${encodeURIComponent(file.name)}`}
                className="block w-full text-left px-4 py-2 rounded transition-colors hover:bg-[rgb(var(--color-primary)/0.08)] truncate"
                title={file.name}
                style={{ maxWidth: "100%" }}
              >
                {file.name}
              </Link>
            </li>
          ))}
        </ul>
        <Link
          to="/buckets"
          className="mt-6 text-sm text-[rgb(var(--color-primary)/1)] hover:underline"
        >
          ← Back to Buckets
        </Link>
      </div>
    </section>
  );
}
