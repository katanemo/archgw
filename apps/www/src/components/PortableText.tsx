import { PortableText as SanityPortableText } from "@portabletext/react";
import Image from "next/image";
import { urlFor } from "@/lib/sanity";
import type { PortableTextBlock } from "@portabletext/types";

interface PortableTextProps {
  content: PortableTextBlock[];
}

const components = {
  types: {
    image: ({ value }: any) => {
      if (!value?.asset) return null;

      const imageUrl = urlFor(value);
      const asset = value.asset;

      // Get natural dimensions if available from metadata
      const dimensions = asset.metadata?.dimensions;
      const width = dimensions?.width || 1000;
      const height = dimensions?.height || 562;
      const aspectRatio = dimensions ? height / width : 0.5625; // Default to 16:9 if no dimensions

      return (
        <div className="my-6 lg:my-8">
          <div className="max-w-3xl mx-auto">
            <div className="relative w-full overflow-hidden rounded-lg bg-black/5">
              <div
                className="relative w-full"
                style={{ paddingBottom: `${aspectRatio * 100}%` }}
              >
                <Image
                  src={imageUrl.width(Math.min(width, 1000)).url()}
                  alt={value.alt || "Blog image"}
                  fill
                  className="object-contain"
                  sizes="(max-width: 768px) 100vw, (max-width: 1200px) 768px, 1000px"
                />
              </div>
            </div>
            {value.alt && (
              <p className="mt-2 text-sm text-black/60 text-center">
                {value.alt}
              </p>
            )}
          </div>
        </div>
      );
    },
  },
  block: {
    h1: (props: any) => (
      <h1 className="text-4xl sm:text-5xl lg:text-6xl font-normal leading-tight tracking-tighter text-black mt-8 mb-4 first:mt-0">
        <span className="font-sans">{props.children}</span>
      </h1>
    ),
    h2: (props: any) => (
      <h2 className="text-3xl sm:text-4xl lg:text-5xl font-normal leading-tight tracking-tighter text-black mt-8 mb-4 first:mt-0">
        <span className="font-sans">{props.children}</span>
      </h2>
    ),
    h3: (props: any) => (
      <h3 className="text-2xl sm:text-3xl lg:text-4xl font-normal leading-tight tracking-tighter text-black mt-6 mb-3 first:mt-0">
        <span className="font-sans">{props.children}</span>
      </h3>
    ),
    h4: (props: any) => (
      <h4 className="text-xl sm:text-2xl lg:text-3xl font-normal leading-tight tracking-tighter text-black mt-6 mb-3 first:mt-0">
        <span className="font-sans">{props.children}</span>
      </h4>
    ),
    normal: (props: any) => (
      <p className="text-base sm:text-lg font-sans font-[400] tracking-[-0.5px] text-black/80 mb-4 leading-relaxed">
        {props.children}
      </p>
    ),
    blockquote: (props: any) => (
      <blockquote className="border-l-4 border-[var(--secondary)] pl-6 py-2 my-6 italic text-black/70">
        {props.children}
      </blockquote>
    ),
  },
  list: {
    bullet: (props: any) => (
      <ul className="list-disc list-inside mb-4 space-y-2 text-base sm:text-lg font-sans font-[400] tracking-[-0.5px] text-black/80">
        {props.children}
      </ul>
    ),
    number: (props: any) => (
      <ol className="list-decimal list-inside mb-4 space-y-2 text-base sm:text-lg font-sans font-[400] tracking-[-0.5px] text-black/80">
        {props.children}
      </ol>
    ),
  },
  listItem: {
    bullet: (props: any) => <li className="ml-4">{props.children}</li>,
    number: (props: any) => <li className="ml-4">{props.children}</li>,
  },
  marks: {
    strong: ({ children }: { children: React.ReactNode }) => (
      <strong className="font-semibold text-black">{children}</strong>
    ),
    em: ({ children }: { children: React.ReactNode }) => (
      <em className="italic">{children}</em>
    ),
    link: (props: any) => (
      <a
        href={props.value?.href || "#"}
        target={props.value?.href?.startsWith("http") ? "_blank" : undefined}
        rel={
          props.value?.href?.startsWith("http")
            ? "noopener noreferrer"
            : undefined
        }
        className="text-[var(--secondary)] hover:underline font-medium"
      >
        {props.children}
      </a>
    ),
  },
};

export function PortableText({ content }: PortableTextProps) {
  return <SanityPortableText value={content} components={components} />;
}
