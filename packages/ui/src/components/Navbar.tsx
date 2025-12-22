"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import { Logo } from "./Logo";
import { Button } from "./ui/button";
import { cn } from "../lib/utils";
import { motion, AnimatePresence } from "framer-motion";
import { X, Menu } from "lucide-react";

const navItems = [
  { href: "https://docs.planoai.dev/get_started/quickstart", label: "start locally" },
  { href: "https://docs.planoai.dev", label: "docs" },
  { href: "/research", label: "research" },
  { href: "/blog", label: "blog" },
  { href: "/contact", label: "contact" },
];

export function Navbar() {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [isDarkBackground, setIsDarkBackground] = useState(false);

  // Detect background color behind dropdown menu
  useEffect(() => {
    if (!isMenuOpen) {
      setIsDarkBackground(false);
      return;
    }

    const detectBackground = () => {
      // Small delay to ensure DOM is ready
      setTimeout(() => {
        const nav = document.querySelector("nav");
        if (!nav) return;

        const navRect = nav.getBoundingClientRect();
        const dropdownBottom = navRect.bottom;
        const checkY = dropdownBottom + 20; // Just below the dropdown

        // First, try to find section elements directly
        const main = document.querySelector("main");
        if (main) {
          const sections = main.querySelectorAll("section");
          let foundDarkSection = false;

          sections.forEach((section) => {
            const rect = section.getBoundingClientRect();
            // Check if this section is visible below the navbar
            if (rect.top <= checkY && rect.bottom > checkY) {
              // Check for dark background classes
              const classList = Array.from(section.classList);
              const hasDarkBg = classList.some(
                (cls) =>
                  cls.includes("bg-[#1a1a1a]") ||
                  cls.includes("bg-black") ||
                  cls.includes("bg-gray-900") ||
                  cls.includes("bg-neutral-900") ||
                  cls.includes("dark"),
              );

              if (hasDarkBg) {
                foundDarkSection = true;
                setIsDarkBackground(true);
                return;
              }

              // Also check computed background
              const computed = window.getComputedStyle(section);
              const bg = computed.backgroundColor;
              if (bg && bg !== "rgba(0, 0, 0, 0)" && bg !== "transparent") {
                const rgbMatch = bg.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
                if (rgbMatch) {
                  const r = parseInt(rgbMatch[1]);
                  const g = parseInt(rgbMatch[2]);
                  const b = parseInt(rgbMatch[3]);
                  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
                  setIsDarkBackground(luminance < 0.5);
                  foundDarkSection = true;
                  return;
                }
              }
            }
          });

          if (foundDarkSection) return;
        }

        // Fallback: Check element at point
        const centerX = window.innerWidth / 2;
        const elementBelow = document.elementFromPoint(centerX, checkY);

        if (elementBelow) {
          let current: HTMLElement | null = elementBelow as HTMLElement;
          let backgroundColor = "";

          // Walk up the DOM tree
          let levels = 0;
          while (
            current &&
            !backgroundColor &&
            current !== document.body &&
            levels < 15
          ) {
            const computed = window.getComputedStyle(current);
            const bg = computed.backgroundColor;

            if (bg && bg !== "rgba(0, 0, 0, 0)" && bg !== "transparent") {
              const rgbaMatch = bg.match(
                /rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/,
              );
              if (rgbaMatch) {
                const alpha = rgbaMatch[4] ? parseFloat(rgbaMatch[4]) : 1;
                if (alpha > 0.1) {
                  backgroundColor = bg;
                  break;
                }
              } else {
                backgroundColor = bg;
                break;
              }
            }

            current = current.parentElement;
            levels++;
          }

          if (!backgroundColor) {
            const bodyBg = window.getComputedStyle(
              document.body,
            ).backgroundColor;
            backgroundColor = bodyBg;
          }

          if (backgroundColor) {
            const rgbMatch = backgroundColor.match(
              /rgba?\((\d+),\s*(\d+),\s*(\d+)/,
            );
            if (rgbMatch) {
              const r = parseInt(rgbMatch[1]);
              const g = parseInt(rgbMatch[2]);
              const b = parseInt(rgbMatch[3]);
              const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
              setIsDarkBackground(luminance < 0.5);
            } else {
              const darkColors = [
                "black",
                "#000",
                "#000000",
                "rgb(0,0,0)",
                "rgba(0,0,0",
                "#1a1a1a",
              ];
              const isDark = darkColors.some((color) =>
                backgroundColor.toLowerCase().includes(color.toLowerCase()),
              );
              setIsDarkBackground(isDark);
            }
          }
        }
      }, 100);
    };

    // Detect on open and on scroll
    detectBackground();
    const scrollHandler = () => detectBackground();
    const resizeHandler = () => detectBackground();

    window.addEventListener("scroll", scrollHandler, { passive: true });
    window.addEventListener("resize", resizeHandler);

    return () => {
      window.removeEventListener("scroll", scrollHandler);
      window.removeEventListener("resize", resizeHandler);
    };
  }, [isMenuOpen]);

  // Close menu when route changes
  const handleLinkClick = () => {
    setIsMenuOpen(false);
  };

  return (
    <nav className="relative z-50 bg-gradient-to-b from-transparent to-white/5 backdrop-blur border-b border-neutral-200/5">
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
                  "font-mono tracking-tighter",
                )}
              >
                {item.label}
              </Link>
            ))}
          </div>

          {/* Mobile Menu Button */}
          <div className="md:hidden">
            <button
              onClick={(e) => {
                e.stopPropagation();
                setIsMenuOpen(!isMenuOpen);
              }}
              className="p-2 rounded-md text-[var(--muted)] hover:text-[var(--primary)] transition-colors"
              aria-label="Toggle menu"
              aria-expanded={isMenuOpen}
            >
              <AnimatePresence mode="wait" initial={false}>
                {isMenuOpen ? (
                  <motion.div
                    key="close"
                    initial={{ opacity: 0, rotate: -90 }}
                    animate={{ opacity: 1, rotate: 0 }}
                    exit={{ opacity: 0, rotate: 90 }}
                    transition={{ duration: 0.2 }}
                  >
                    <X className="h-6 w-6" />
                  </motion.div>
                ) : (
                  <motion.div
                    key="menu"
                    initial={{ opacity: 0, rotate: 90 }}
                    animate={{ opacity: 1, rotate: 0 }}
                    exit={{ opacity: 0, rotate: -90 }}
                    transition={{ duration: 0.2 }}
                  >
                    <Menu className="h-6 w-6" />
                  </motion.div>
                )}
              </AnimatePresence>
            </button>
          </div>
        </div>
      </div>

      {/* Mobile Dropdown Menu - Outside constrained container for full width */}
      <AnimatePresence>
        {isMenuOpen && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
            className="md:hidden overflow-hidden bg-[#7580DF]/70"
          >
            <div className="max-w-[85rem] mx-auto px-6 lg:px-8 py-3">
              <div className="flex flex-col gap-0.5">
                {navItems.map((item, index) => (
                  <motion.div
                    key={item.href}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{
                      duration: 0.3,
                      delay: index * 0.05,
                      ease: [0.16, 1, 0.3, 1],
                    }}
                  >
                    <Link
                      href={item.href}
                      onClick={handleLinkClick}
                      className={cn(
                        "block px-0 py-1.5 border-b border-dashed transition-colors font-mono tracking-tighter",
                        "text-sm font-medium",
                        "text-white",
                      )}
                    >
                      {item.label}
                    </Link>
                  </motion.div>
                ))}
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </nav>
  );
}
