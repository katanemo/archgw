"use client";

import React from "react";
import Link from "next/link";
import { Logo } from "./Logo";
import { Button } from "./ui/button";
import { cn } from "@/lib/utils";

const navItems = [
  { href: "/start", label: "start locally" },
  { href: "/docs", label: "docs" },
  { href: "/model-research", label: "models research" },
  { href: "/blog", label: "blog" },
  { href: "/why", label: "why plano?" },
];

export function Navbar() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 bg-gradient-to-b from-transparent to-white/5 backdrop-blur border-b border-neutral-200/5">
      <div className="max-w-[85rem] mx-auto px-6 lg:px-8">
        <div className="flex items-center justify-between h-20">
          {/* Logo */}
          <Link href="/" className="flex items-center">
            <Logo />
          </Link>

          {/* Navigation Links and CTA - Far Right */}
          <div className="hidden md:flex items-center justify-end gap-8">
            {navItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "text-lg font-medium text-[var(--muted)]",
                  "hover:text-[var(--primary)] transition-colors",
                  "font-mono tracking-tighter"
                )}
              >
                {item.label}
              </Link>
            ))}
          </div>

          {/* Mobile Menu Button */}
          <div className="md:hidden">
            <button
              className="p-2 rounded-md text-[var(--muted)] hover:text-[var(--primary)]"
              aria-label="Toggle menu"
            >
              <svg
                className="h-6 w-6"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4 6h16M4 12h16M4 18h16"
                />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </nav>
  );
}

