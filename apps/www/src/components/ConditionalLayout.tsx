"use client";

import { usePathname } from "next/navigation";
import { Navbar, Footer } from "@katanemo/ui";

export function ConditionalLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isStudio = pathname?.startsWith("/studio");

  if (isStudio) {
    return <>{children}</>;
  }

  return (
    <div className="min-h-screen">
      <Navbar />
      <main className="pt-2 md:pt-10">{children}</main>
      <Footer />
    </div>
  );
}
