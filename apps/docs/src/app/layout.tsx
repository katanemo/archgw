import type { Metadata } from "next";
import "./globals.css";
import "@katanemo/shared-styles/globals.css";
import { Navbar, Footer } from "@katanemo/ui";

export const metadata: Metadata = {
  title: "Plano Documentation",
  description: "Documentation for Plano - The AI-native network for agents",
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
      </body>
    </html>
  );
}

