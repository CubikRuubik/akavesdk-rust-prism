import { useRef } from "react";
import { useUploadFile } from "../hooks/useAkave";

export function FileUploadButton({
  bucketName,
  onSuccess,
}: {
  bucketName: string;
  onSuccess?: () => void;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const uploadFile = useUploadFile();

  const handleButtonClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const arrayBuffer = await file.arrayBuffer();
      await uploadFile.mutateAsync({
        bucket_name: bucketName,
        file_name: file.name,
        file_content: new Uint8Array(arrayBuffer),
      });
      if (onSuccess) onSuccess();
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
    } catch (_err) {
      // Optionally handle error here
    } finally {
      // Reset input so the same file can be selected again
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  return (
    <>
      <button
        type="button"
        className="
          px-4 py-2 rounded font-semibold transition-colors border
          bg-[rgb(var(--color-secondary)/0.7)] 
          dark:bg-[rgb(var(--color-secondary)/0.7)]
          text-[rgb(var(--color-text)/1)]
          border-[rgb(var(--color-primary)/0.2)]
          hover:bg-[rgb(var(--color-secondary)/1)]
          hover:border-[rgb(var(--color-primary)/0.5)]
          flex items-center gap-2 cusrsor-pointer mb-4
        "
        onClick={handleButtonClick}
        disabled={uploadFile.isPending}
      >
        {uploadFile.isPending ? "Uploading..." : "Upload new file"}
      </button>
      <input
        ref={fileInputRef}
        type="file"
        className="hidden"
        onChange={handleFileChange}
        disabled={uploadFile.isPending}
      />
      {uploadFile.error && (
        <div className="text-red-500 text-sm mt-2">
          {(uploadFile.error as Error).message}
        </div>
      )}
    </>
  );
}
