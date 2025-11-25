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
};

export default nextConfig;

