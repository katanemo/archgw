import type { Metadata } from "next";
import "@katanemo/shared-styles/globals.css";
import { Analytics } from "@vercel/analytics/next";
import { Navbar, Footer } from "@katanemo/ui";

export const metadata: Metadata = {
  title: "Plano - The AI-native network for agents",
  description:
    "Build and scale AI agents without handling the low-level plumbing.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        <div className="min-h-screen">
          <Navbar />
          <main className="pt-20">{children}</main>
          <Footer />
        </div>
        <Analytics />
      </body>
    </html>
  );
}
