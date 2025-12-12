import Link from "next/link";

export default function NotFound() {
  return (
    <div className="min-h-screen bg-white flex items-center justify-center">
      <div className="max-w-md mx-auto px-4 text-center">
        <h1 className="text-4xl sm:text-5xl font-normal leading-tight tracking-tighter text-black mb-4">
          <span className="font-sans">Post Not Found</span>
        </h1>
        <p className="text-lg font-sans font-[400] tracking-[-0.5px] text-black/70 mb-8">
          The blog post you're looking for doesn't exist or has been removed.
        </p>
        <Link
          href="/blog"
          className="inline-flex items-center gap-2 text-base font-medium text-black hover:text-[var(--secondary)] transition-colors"
        >
          <svg
            className="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          Back to Blog
        </Link>
      </div>
    </div>
  );
}
