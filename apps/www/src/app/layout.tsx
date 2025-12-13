import type { Metadata } from "next";
import "@katanemo/shared-styles/globals.css";
import { Analytics } from "@vercel/analytics/next";
import { ConditionalLayout } from "@/components/ConditionalLayout";

export const metadata: Metadata = {
  title: "Plano - Delivery Infrastructure for Agentic Apps",
  description:
    "Build agents faster, and deliver them reliably to production - by offloading the critical plumbing work to Plano.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        <ConditionalLayout>{children}</ConditionalLayout>
        <Analytics />
      </body>
    </html>
  );
}
