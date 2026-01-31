// src/app/layout.tsx

import "./globals.css";

import { Fraunces, Space_Grotesk } from "next/font/google";

import type { Metadata } from "next";

const spaceGrotesk = Space_Grotesk({
  variable: "--font-ui",
  subsets: ["latin"],
  display: "swap",
});

const fraunces = Fraunces({
  variable: "--font-display",
  subsets: ["latin"],
  display: "swap",
});

export const metadata: Metadata = {
  title: "연세대학교 공지 뷰어",
  description: "학교의 모든 공지사항을 한 곳에서 확인하세요.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ko">
      <body
        className={`${spaceGrotesk.variable} ${fraunces.variable} antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
