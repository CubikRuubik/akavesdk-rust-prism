import { useAccount } from "wagmi";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { Link } from "react-router-dom";
import { useListBuckets } from "../hooks/useAkave";

export default function Home() {
  const { address } = useAccount();
  const { data: buckets, isLoading, error } = useListBuckets(!!address);

  if (!address) {
    return (
      <section className="flex flex-col items-center justify-center flex-1 py-24">
        <div className="bg-[rgb(var(--color-secondary)/1)] rounded-xl shadow-lg p-10 flex flex-col items-center min-w-[320px] max-w-lg w-full">
          <p className="mb-4 text-[rgb(var(--color-text)/0.8)]">
            Connect your wallet to view your buckets.
          </p>
          <ConnectButton />
        </div>
      </section>
    );
  }

  return (
    <section className="flex flex-col items-center justify-center flex-1 py-24">
      <div className="bg-[rgb(var(--color-secondary)/1)] rounded-xl shadow-lg p-10 flex flex-col items-center min-w-[320px] max-w-lg w-full">
        <h1 className="text-2xl font-bold mb-6 text-[rgb(var(--color-primary)/1)]">
          Buckets
        </h1>
        {isLoading && <div>Loading buckets...</div>}
        {error && (
          <div className="text-red-500">{(error as Error).message}</div>
        )}
        <ul className="space-y-2 w-full">
          {buckets?.buckets.map((bucket) => (
            <li key={bucket.id}>
              <Link
                to={`/buckets/${encodeURIComponent(bucket.id)}`}
                className="block w-full text-left px-4 py-2 rounded transition-colors hover:bg-[rgb(var(--color-primary)/0.08)]"
              >
                {bucket.name}
              </Link>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
