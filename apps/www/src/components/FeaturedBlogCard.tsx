"use client";

import { motion } from "framer-motion";
import Image from "next/image";
import Link from "next/link";
import { urlFor } from "@/lib/sanity";

interface FeaturedBlogCardProps {
  post: {
    _id: string;
    title: string;
    slug: { current: string };
    summary?: string;
    formattedDate?: string;
    mainImage?: any;
    mainImageUrl?: string;
    author?: {
      name?: string;
      title?: string;
      image?: any;
    };
  };
}

export function FeaturedBlogCard({ post }: FeaturedBlogCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        duration: 0.4,
        ease: "easeOut",
      }}
    >
      <Link href={`/blog/${post.slug.current}`} className="group block">
        <motion.div
          className="bg-linear-to-b from-primary/20 to-primary/1 border border-primary/20 rounded-md p-8 sm:p-10 lg:p-12"
          whileHover={{
            borderColor: "rgba(119, 128, 217, 0.5)",
          }}
          transition={{
            duration: 0.2,
            ease: "easeOut",
          }}
        >
          <div className="grid lg:grid-cols-2 gap-8 lg:gap-12 items-center">
            {/* Content */}
            <div className="order-1 text-left">
              {post.formattedDate && (
                <div className="text-base font-medium tracking-[-0.9px] text-black mb-4">
                  {post.formattedDate}
                </div>
              )}
              <h2 className="text-3xl sm:text-4xl lg:text-4xl font-medium tracking-[-1.5px] text-black mb-4 group-hover:text-[var(--secondary)] transition-colors text-left">
                <span className="font-sans">{post.title}</span>
              </h2>
              {post.summary && (
                <p className="text-base sm:text-base font-mono font-normal tracking-[-0.9px] text-black/70 mb-6 text-left">
                  {post.summary}
                </p>
              )}
              {post.author && (
                <div className="flex items-center gap-3">
                  {post.author.image ? (
                    <div className="relative w-12 h-12 rounded overflow-hidden shrink-0">
                      <Image
                        src={urlFor(post.author.image).width(80).url()}
                        alt={post.author.name || "Author"}
                        fill
                        className="object-cover"
                      />
                    </div>
                  ) : (
                    <div className="w-12 h-12 rounded-full bg-[var(--secondary)]/20 shrink-0" />
                  )}
                  <div>
                    {post.author.name && (
                      <div className="text-lg font-mono font-semibold tracking-wider text-primary uppercase">
                        {post.author.name}
                      </div>
                    )}
                    {post.author.title && (
                      <div className="text-sm font-mono font-normal tracking-wider text-[#28327D] uppercase">
                        {post.author.title}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>

            {/* Image */}
            <div className="relative aspect-[18/9] w-full overflow-hidden rounded-lg bg-black/5 order-2">
              {post.mainImage ? (
                <Image
                  src={urlFor(post.mainImage).width(800).url()}
                  alt={post.title}
                  fill
                  className="object-cover"
                />
              ) : post.mainImageUrl ? (
                <Image
                  src={post.mainImageUrl}
                  alt={post.title}
                  fill
                  className="object-cover"
                />
              ) : (
                <div className="w-full h-full bg-gradient-to-br from-[var(--secondary)]/20 to-black/10" />
              )}
            </div>
          </div>
        </motion.div>
      </Link>
    </motion.div>
  );
}
