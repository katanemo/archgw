import type { Metadata } from "next";
import Script from "next/script";
import "@katanemo/shared-styles/globals.css";
import { Analytics } from "@vercel/analytics/next";
import { ConditionalLayout } from "@/components/ConditionalLayout";

export const metadata: Metadata = {
  title: "Plano - Delivery Infrastructure for Agentic Apps",
  description:
    "Build agents faster, and deliver them reliably to production - by offloading the critical plumbing work to Plano!",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        {/* Google tag (gtag.js) */}
        <Script
          src="https://www.googletagmanager.com/gtag/js?id=G-6J5LQH3Q9G"
          strategy="afterInteractive"
        />
        <Script id="google-analytics" strategy="afterInteractive">
          {`
            window.dataLayer = window.dataLayer || [];
            function gtag(){dataLayer.push(arguments);}
            gtag('js', new Date());
            gtag('config', 'G-6J5LQH3Q9G');
          `}
        </Script>
        <ConditionalLayout>{children}</ConditionalLayout>
        <Analytics />
      </body>
    </html>
  );
}
