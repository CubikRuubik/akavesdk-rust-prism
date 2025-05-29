import { Link } from "react-router-dom";

export default function NotFound() {
  return (
    <section className="flex flex-col items-center justify-center flex-1 py-24">
      <div className="bg-[rgb(var(--color-secondary)/1)] dark:bg-[rgb(var(--color-secondary)/1)] rounded-xl shadow-lg p-10 flex flex-col items-center">
        <h1 className="text-6xl font-extrabold text-[rgb(var(--color-primary)/1)] mb-4">
          404
        </h1>
        <h2 className="text-2xl font-bold mb-2 text-[rgb(var(--color-text)/1)]">
          Page Not Found
        </h2>
        <p className="mb-6 text-center text-[rgb(var(--color-text)/0.8)]">
          Sorry, the page you are looking for does not exist or has been moved.
        </p>
        <Link to="/" className="btn">
          Go Home
        </Link>
      </div>
    </section>
  );
}
