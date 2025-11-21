import React from "react";
import Image from "next/image";

export function Logo() {
  return (
    <div className="flex items-center">
      {/* LogoMarkSquare SVG */}
      <Image
        src="/Logomark.svg"
        alt="Plano Logo"
        width={90}
        height={20}
        className="flex-shrink-0"
      />
    </div>
  );
}
