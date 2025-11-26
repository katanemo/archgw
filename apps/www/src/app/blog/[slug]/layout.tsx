import { Metadata } from "next";
import { client } from "@/lib/sanity";

type Params = Promise<{ slug: string }>;

interface BlogPost {
  _id: string;
  title: string;
  slug: { current: string };
  summary?: string;
  publishedAt?: string;
  author?: {
    name?: string;
    title?: string;
    image?: any;
  };
}

async function getBlogPost(slug: string): Promise<BlogPost | null> {
  const query = `*[_type == "blog" && slug.current == $slug && published == true][0] {
    _id,
    title,
    slug,
    summary,
    publishedAt,
    author
  }`;

  const post = await client.fetch(query, { slug });
  return post || null;
}

export async function generateMetadata({
  params,
}: {
  params: Params;
}): Promise<Metadata> {
  try {
    const resolvedParams = await params;
    const post = await getBlogPost(resolvedParams.slug);

    if (!post) {
      return {
        title: "Post Not Found - Plano",
        description: "The requested blog post could not be found.",
      };
    }

    const baseUrl =
      process.env.NEXT_PUBLIC_BASE_URL ||
      (process.env.VERCEL_URL
        ? `https://${process.env.VERCEL_URL}`
        : "http://localhost:3000");

    const ogImageUrl = `${baseUrl}/api/og/${resolvedParams.slug}`;

    const metadata: Metadata = {
      title: `${post.title} - Plano Blog`,
      description: post.summary || "Read more on Plano Blog",
      openGraph: {
        title: post.title,
        description: post.summary || "Read more on Plano Blog",
        type: "article",
        publishedTime: post.publishedAt,
        authors: post.author?.name ? [post.author.name] : undefined,
        url: `${baseUrl}/blog/${resolvedParams.slug}`,
        siteName: "Plano",
        images: [
          {
            url: ogImageUrl,
            width: 1200,
            height: 630,
            alt: post.title,
          },
        ],
        locale: "en_US",
      },
      twitter: {
        card: "summary_large_image",
        title: post.title,
        description: post.summary || "Read more on Plano Blog",
        images: [ogImageUrl],
      },
    };

    return metadata;
  } catch (error) {
    console.error("Error generating metadata:", error);
    return {
      title: "Blog Post - Plano",
      description: "Read this post on Plano Blog",
    };
  }
}

interface LayoutProps {
  children: React.ReactNode;
  params: Params;
}

export default async function Layout({ children, params }: LayoutProps) {
  return <>{children}</>;
}

