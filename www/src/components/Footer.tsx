import React from "react";
import Link from "next/link";
import Image from "next/image";

const footerLinks = {
  company: [
    { label: "Product", href: "/product" },
    { label: "Use Cases", href: "/use-cases" },
    { label: "Blog", href: "/blog" },
    { label: "Plano LLMs", href: "/llms" }
  ],
  developerResources: [
    { label: "Documentation", href: "/docs" }
  ]
};

export function Footer() {
  return (
    <footer className="relative overflow-hidden py-24 px-6 lg:px-[102px] pb-48" style={{ background: 'linear-gradient(to top right, #ffffff, #dcdfff)' }}>
      <div className="max-w-[81rem] mx-auto relative z-10">
        {/* Main Grid Layout */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-20">
          {/* Left Column - Tagline and Copyright */}
          <div className="flex flex-col">
            <p className="font-sans text-2xl text-black tracking-[-1.7px]! leading-8 mb-8">
              Plano is the powerful, intelligent platform that empowers teams to seamlessly build, automate, and scale agentic systems with ease.
            </p>
            
            {/* Copyright */}
            <div className="mt-auto">
              <p className="font-sans text-base text-black/63 tracking-[-0.8px]!">
                © Katanemo Labs, Inc. 2025 / Plano by Katanemo Labs, Inc.
              </p>
            </div>
          </div>

          {/* Right Column - Navigation Links */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-8">
            {/* Company Links */}
            <div>
              <h3 className="font-sans font-medium text-3xl text-black tracking-[-1.6px]! mb-6">
                Company
              </h3>
              <nav className="space-y-4">
                {footerLinks.company.map((link) => (
                  <Link
                    key={link.href}
                    href={link.href}
                    className="block font-sans text-xl text-black tracking-[-1px]! hover:text-[var(--primary)] transition-colors"
                  >
                    {link.label}
                  </Link>
                ))}
              </nav>
            </div>

            {/* Developer Resources */}
            <div>
              <h3 className="font-sans font-medium text-3xl text-black tracking-[-1.6px]! mb-6">
                Developer Resources
              </h3>
              <nav className="space-y-4">
                {footerLinks.developerResources.map((link) => (
                  <Link
                    key={link.href}
                    href={link.href}
                    className="block font-sans text-xl text-black tracking-[-1px]! hover:text-[var(--primary)] transition-colors"
                  >
                    {link.label}
                  </Link>
                ))}
              </nav>
            </div>
          </div>
        </div>
      </div>

      {/* Half-Cut Plano Logo Background */}
      <div className="absolute bottom-0 left-0 right-0 overflow-hidden pointer-events-none">
        <div className="max-w-[81rem] mx-auto px-6 lg:px-[1px]">
          <div className="relative w-full flex justify-start">
            <Image
              src="/LogoOutline.svg"
              alt="Plano Logo"
              width={1800}
              height={200}
              className="w-150 h-auto opacity-30 select-none"
              style={{
                transform: 'translateY(0%)', // Push logo down more while showing top part
                transformOrigin: 'center bottom'
              }}
            />
          </div>
        </div>
      </div>
    </footer>
  );
}
