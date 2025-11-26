import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  transpilePackages: [
    "@katanemo/ui",
    "@katanemo/shared-styles",
    "@katanemo/tailwind-config",
    "@katanemo/tsconfig",
  ],
  experimental: {
    // Ensure workspace packages are handled correctly
    externalDir: true,
  },
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "cdn.sanity.io",
        port: "",
        pathname: "/**",
      },
    ],
  },
};

export default nextConfig;
