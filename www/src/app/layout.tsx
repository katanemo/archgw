import type { Metadata } from "next";
import "./globals.css";

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
      <body className="antialiased">{children}</body>
    </html>
  );
}
