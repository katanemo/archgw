import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  transpilePackages: [
    "@katanemo/ui",
    "@katanemo/shared-styles",
    "@katanemo/tailwind-config",
    "@katanemo/tsconfig",
  ],
};

export default nextConfig;

