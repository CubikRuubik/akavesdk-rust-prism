import { useState } from "react";
import { useCreateBucket } from "../hooks/useAkave";

export function AddBucketButton({ onSuccess }: { onSuccess?: () => void }) {
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const createBucket = useCreateBucket();

  const handleAddBucket = async () => {
    setError(null);
    const bucketName = window.prompt("Enter a name for your new bucket:");
    if (!bucketName) return;
    setCreating(true);
    try {
      await createBucket.mutateAsync(bucketName);
      if (onSuccess) onSuccess();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setCreating(false);
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
          flex items-center gap-2 cusrsor-pointer
          mb-4
        "
        onClick={handleAddBucket}
        disabled={creating}
      >
        {creating ? "Creating..." : "Add Bucket"}
      </button>
      {error && <div className="text-red-500 text-sm mt-2">{error}</div>}
      {createBucket.error && (
        <div className="text-red-500 text-sm mt-2">
          {(createBucket.error as Error).message}
        </div>
      )}
    </>
  );
}
