"use client";

import { motion } from "framer-motion";
import Image from "next/image";
import Link from "next/link";
import { urlFor } from "@/lib/sanity";

interface BlogCardProps {
  post: {
    _id: string;
    title: string;
    slug: { current: string };
    formattedDate?: string;
    author?: {
      name?: string;
      title?: string;
      image?: any;
    };
  };
  index?: number;
}

export function BlogCard({ post, index = 0 }: BlogCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        duration: 0.4,
        delay: index * 0.05,
        ease: "easeOut",
      }}
    >
      <Link href={`/blog/${post.slug.current}`} className="group block h-full">
        <motion.article
          className="h-full min-h-[320px] bg-linear-to-b from-primary/20 to-primary/1 border border-primary/20 rounded-md p-6 sm:p-8 flex flex-col"
          whileHover={{
            borderColor: "rgba(119, 128, 217, 0.5)",
          }}
          transition={{
            duration: 0.2,
            ease: "easeOut",
          }}
        >
          {post.formattedDate && (
            <div className="text-base font-medium tracking-[-0.9px] text-black mb-6">
              {post.formattedDate}
            </div>
          )}
          <h3 className="text-xl sm:text-2xl font-normal leading-tight tracking-tighter text-black group-hover:text-[var(--secondary)] transition-colors flex-grow">
            <span className="font-sans font-medium tracking-[-1.5px]">
              {post.title}
            </span>
          </h3>
          {post.author && (
            <div className="flex items-center gap-3 mt-auto pt-6">
              {post.author.image ? (
                <div className="relative w-10 h-10 rounded overflow-hidden shrink-0">
                  <Image
                    src={urlFor(post.author.image).width(80).url()}
                    alt={post.author.name || "Author"}
                    fill
                    className="object-cover"
                  />
                </div>
              ) : (
                <div className="w-10 h-10 rounded-full bg-[var(--secondary)]/20 shrink-0" />
              )}
              <div>
                {post.author.name && (
                  <div className="text-base font-mono font-semibold tracking-wider text-primary uppercase">
                    {post.author.name}
                  </div>
                )}
                {post.author.title && (
                  <div className="text-xs font-mono font-normal tracking-wider text-[#28327D] uppercase">
                    {post.author.title}
                  </div>
                )}
              </div>
            </div>
          )}
        </motion.article>
      </Link>
    </motion.div>
  );
}
