import React from "react";
import Image from "next/image";

const customerLogos = [
  {
    name: "HuggingFace",
    src: "/logos/huggingface.svg",
  },
  {
    name: "T-Mobile",
    src: "/logos/tmobile.svg",
  },
  {
    name: "Chase",
    src: "/logos/chase.svg",
  },
  {
    name: "SanDisk",
    src: "/logos/sandisk.svg",
  },
  {
    name: "Oracle",
    src: "/logos/oracle.svg",
  },
];

export function LogoCloud() {
  return (
    <section className="relative py-6 sm:py-8 px-4 sm:px-6 lg:px-8">
      <div className="max-w-[81rem] mx-auto">
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4 sm:gap-6 md:gap-8 lg:gap-12 items-center justify-items-center">
          {customerLogos.map((logo, index) => {
            const isLast = index === customerLogos.length - 1;
            return (
              <div
                key={logo.name}
                className={`flex items-center justify-center opacity-60 hover:opacity-80 transition-opacity duration-300 w-full max-w-32 sm:max-w-40 md:max-w-48 h-10 sm:h-12 md:h-16 ${
                  isLast ? "col-span-2 md:col-span-3 lg:col-span-1" : ""
                }`}
              >
                <Image
                  src={logo.src}
                  alt={`${logo.name} logo`}
                  width={128}
                  height={40}
                  className="w-full h-full object-contain filter grayscale hover:grayscale-0 transition-all duration-300"
                />
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
